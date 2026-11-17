use sqlx::{Connection, SqlitePool};

use crate::{
    db,
    domain::{GroupId, InvitationId, InvitationToken, MAX_GROUP_MEMBERS, UserId},
    services::{
        members::{self, MemberSummary},
        membership::{self, MembershipError},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedInvitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub token: InvitationToken,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationPreview {
    pub group_id: GroupId,
    pub group_name: String,
    pub members: Vec<MemberSummary>,
    pub member_count: i64,
    pub is_full: bool,
    pub is_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinOutcome {
    Joined {
        group_id: GroupId,
        group_name: String,
    },
    AlreadyMember {
        group_id: GroupId,
        group_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeOutcome {
    Revoked { group_name: String },
    AlreadyRevoked { group_name: String },
}

#[derive(Debug, thiserror::Error)]
pub enum InvitationError {
    #[error("user {0} is not the owner of this group")]
    NotOwner(UserId),
    #[error("group {0} does not exist")]
    GroupNotFound(GroupId),
    #[error("invitation does not exist")]
    InvitationNotFound,
    #[error("invitation token is invalid")]
    InvalidToken,
    #[error("invitation has expired")]
    Expired,
    #[error("invitation has been revoked")]
    Revoked,
    #[error("group is full")]
    GroupFull,
    #[error("invitation expiry must be in the future")]
    InvalidExpiry,
    #[error("stored invitation data is invalid: {0}")]
    CorruptData(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Creates an invitation for an owner. The raw token is returned once and only
/// its hash is stored.
pub async fn create(
    pool: &SqlitePool,
    actor: UserId,
    group: GroupId,
    expires_at: Option<i64>,
) -> Result<CreatedInvitation, InvitationError> {
    let now = super::now_millis();
    if let Some(expiry) = expires_at
        && expiry <= now
    {
        return Err(InvitationError::InvalidExpiry);
    }

    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let owner: Option<i64> = sqlx::query_scalar("SELECT owner_user_id FROM groups WHERE id = ?")
        .bind(group.get())
        .fetch_optional(&mut *transaction)
        .await?;
    let Some(owner) = owner else {
        return Err(InvitationError::GroupNotFound(group));
    };
    if owner != actor.get() {
        return Err(InvitationError::NotOwner(actor));
    }

    let token = InvitationToken::generate();
    let token_hash = token.hash();

    let id_raw: i64 = sqlx::query_scalar(
        "INSERT INTO invitations
             (group_id, token_hash, created_by_user_id, created_at, expires_at, revoked_at)
         VALUES (?, ?, ?, ?, ?, NULL)
         RETURNING id",
    )
    .bind(group.get())
    .bind(&token_hash)
    .bind(actor.get())
    .bind(now)
    .bind(expires_at)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(CreatedInvitation {
        id: InvitationId::new(id_raw)
            .map_err(|error| InvitationError::CorruptData(error.to_string()))?,
        group_id: group,
        token,
        expires_at,
    })
}

/// Reads an invitation preview. Never creates membership.
pub async fn preview(
    pool: &SqlitePool,
    token: &InvitationToken,
    viewer: UserId,
) -> Result<InvitationPreview, InvitationError> {
    let row: Option<PreviewRow> = sqlx::query_as(
        "SELECT i.group_id,
                g.name AS group_name,
                i.expires_at,
                i.revoked_at,
                EXISTS(
                    SELECT 1 FROM group_members m
                    WHERE m.group_id = i.group_id AND m.user_id = ?
                ) AS is_member
         FROM invitations i
         JOIN groups g ON g.id = i.group_id
         WHERE i.token_hash = ?",
    )
    .bind(viewer.get())
    .bind(token.hash())
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(InvitationError::InvalidToken);
    };

    if row.revoked_at.is_some() {
        return Err(InvitationError::Revoked);
    }
    if row
        .expires_at
        .is_some_and(|expiry| expiry <= super::now_millis())
    {
        return Err(InvitationError::Expired);
    }

    let group_id = GroupId::new(row.group_id)
        .map_err(|error| InvitationError::CorruptData(error.to_string()))?;
    let members = members::list_unchecked(pool, group_id)
        .await
        .map_err(map_members_error)?;
    let member_count = members.len() as i64;

    Ok(InvitationPreview {
        group_id,
        group_name: row.group_name,
        members,
        member_count,
        is_full: member_count >= MAX_GROUP_MEMBERS as i64,
        is_member: row.is_member != 0,
    })
}

/// Joins a group after revalidating the invitation, capacity and membership in
/// one write transaction. Repeated joins are idempotent.
pub async fn join(
    pool: &SqlitePool,
    token: &InvitationToken,
    user: UserId,
) -> Result<JoinOutcome, InvitationError> {
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let row: Option<JoinRow> = sqlx::query_as(
        "SELECT i.group_id,
                g.name AS group_name,
                i.expires_at,
                i.revoked_at
         FROM invitations i
         JOIN groups g ON g.id = i.group_id
         WHERE i.token_hash = ?",
    )
    .bind(token.hash())
    .fetch_optional(&mut *transaction)
    .await?;

    let Some(row) = row else {
        return Err(InvitationError::InvalidToken);
    };
    if row.revoked_at.is_some() {
        return Err(InvitationError::Revoked);
    }
    if row
        .expires_at
        .is_some_and(|expiry| expiry <= super::now_millis())
    {
        return Err(InvitationError::Expired);
    }

    let group_id = GroupId::new(row.group_id)
        .map_err(|error| InvitationError::CorruptData(error.to_string()))?;
    let group_name = row.group_name;

    match membership::add_locked(&mut transaction, group_id, user).await {
        Ok(_) => {
            transaction.commit().await?;
            Ok(JoinOutcome::Joined {
                group_id,
                group_name,
            })
        }
        Err(MembershipError::AlreadyMember { .. }) => Ok(JoinOutcome::AlreadyMember {
            group_id,
            group_name,
        }),
        Err(MembershipError::GroupFull { .. }) => Err(InvitationError::GroupFull),
        Err(error) => Err(map_membership_error(error)),
    }
}

/// Revokes an invitation. Idempotent for an already-revoked invitation.
pub async fn revoke(
    pool: &SqlitePool,
    actor: UserId,
    invitation: InvitationId,
) -> Result<RevokeOutcome, InvitationError> {
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let row: Option<RevokeRow> = sqlx::query_as(
        "SELECT g.name AS group_name, g.owner_user_id, i.revoked_at
         FROM invitations i
         JOIN groups g ON g.id = i.group_id
         WHERE i.id = ?",
    )
    .bind(invitation.get())
    .fetch_optional(&mut *transaction)
    .await?;

    let Some(row) = row else {
        return Err(InvitationError::InvitationNotFound);
    };
    if row.owner_user_id != actor.get() {
        return Err(InvitationError::NotOwner(actor));
    }

    if row.revoked_at.is_some() {
        return Ok(RevokeOutcome::AlreadyRevoked {
            group_name: row.group_name,
        });
    }

    sqlx::query("UPDATE invitations SET revoked_at = ? WHERE id = ?")
        .bind(super::now_millis())
        .bind(invitation.get())
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(RevokeOutcome::Revoked {
        group_name: row.group_name,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct PreviewRow {
    group_id: i64,
    group_name: String,
    expires_at: Option<i64>,
    revoked_at: Option<i64>,
    is_member: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct JoinRow {
    group_id: i64,
    group_name: String,
    expires_at: Option<i64>,
    revoked_at: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct RevokeRow {
    group_name: String,
    owner_user_id: i64,
    revoked_at: Option<i64>,
}

fn map_members_error(error: members::MembersError) -> InvitationError {
    match error {
        members::MembersError::Database(error) => InvitationError::Database(error),
        other => InvitationError::CorruptData(other.to_string()),
    }
}

fn map_membership_error(error: MembershipError) -> InvitationError {
    match error {
        MembershipError::Database(error) => InvitationError::Database(error),
        MembershipError::GroupNotFound(_) | MembershipError::UserNotFound(_) => {
            InvitationError::InvitationNotFound
        }
        other => InvitationError::CorruptData(other.to_string()),
    }
}
