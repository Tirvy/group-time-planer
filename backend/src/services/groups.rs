use sqlx::{Connection, SqlitePool};

use crate::{
    db,
    domain::{GroupId, GroupName, MEMBER_COLORS, MemberRole, SetupState, TelegramId, UserId},
};

/// A group with its owner relationship resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: GroupId,
    pub name: GroupName,
    pub owner_user_id: UserId,
    pub created_at: i64,
    pub updated_at: i64,
}
/// Group data as seen by one member, including their role and setup state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupSummary {
    pub id: GroupId,
    pub name: GroupName,
    pub role: MemberRole,
    /// Stable color of the viewer's membership in this group.
    pub color: String,
    pub member_count: i64,
    pub setup_completed_at: Option<i64>,
}

impl GroupSummary {
    pub fn setup_state(&self) -> SetupState {
        SetupState::from_completed_at(self.setup_completed_at)
    }

    pub fn is_owner(&self) -> bool {
        self.role.is_owner()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("user {0} does not exist")]
    UserNotFound(UserId),
    #[error("group {0} does not exist")]
    GroupNotFound(GroupId),
    #[error("group {0} is not available to this user")]
    NotAMember(GroupId),
    #[error("user {0} is not the owner of this group")]
    NotOwner(UserId),
    #[error("target user {0} is not a member of this group")]
    TargetNotMember(UserId),
    #[error("ownership cannot be transferred to the current owner")]
    InvalidTransferTarget,
    #[error("database returned an invalid group id")]
    InvalidGroupId,
    #[error("stored group data is invalid: {0}")]
    CorruptData(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Creates a group and its owner membership in one write transaction.
///
/// The owner is the first member and starts with `setup_pending`. The 20-member
/// cap therefore includes the owner.
pub async fn create_with_owner(
    pool: &SqlitePool,
    name: &GroupName,
    owner: UserId,
) -> Result<Group, GroupError> {
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let owner_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
        .bind(owner.get())
        .fetch_optional(&mut *transaction)
        .await?;
    if owner_exists.is_none() {
        return Err(GroupError::UserNotFound(owner));
    }

    let now = super::now_millis();
    let group_id_raw: i64 = sqlx::query_scalar(
        "INSERT INTO groups (name, owner_user_id, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         RETURNING id",
    )
    .bind(name.as_str())
    .bind(owner.get())
    .bind(now)
    .bind(now)
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO group_members
             (group_id, user_id, role, color, setup_completed_at, joined_at)
         VALUES (?, ?, ?, ?, NULL, ?)",
    )
    .bind(group_id_raw)
    .bind(owner.get())
    .bind(MemberRole::Owner.as_db())
    .bind(MEMBER_COLORS[0])
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    let id = GroupId::new(group_id_raw).map_err(|_| GroupError::InvalidGroupId)?;
    Ok(Group {
        id,
        name: name.clone(),
        owner_user_id: owner,
        created_at: now,
        updated_at: now,
    })
}

/// Lists every group the user belongs to, with the owner's groups first.
///
/// A user may be a member of multiple groups; preferences and setup state are
/// independent per group.
#[derive(Debug, sqlx::FromRow)]
struct SummaryRow {
    id: i64,
    name: String,
    role: String,
    color: String,
    setup_completed_at: Option<i64>,
    member_count: i64,
}

pub async fn list_for_user(
    pool: &SqlitePool,
    user: UserId,
) -> Result<Vec<GroupSummary>, GroupError> {
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "SELECT g.id,
                g.name,
                gm.role,
                gm.color,
                gm.setup_completed_at,
                (SELECT COUNT(*) FROM group_members m WHERE m.group_id = g.id) AS member_count
         FROM group_members gm
         JOIN groups g ON g.id = gm.group_id
         WHERE gm.user_id = ?
         ORDER BY CASE gm.role WHEN 'owner' THEN 0 ELSE 1 END,
                  gm.joined_at ASC,
                  g.id ASC",
    )
    .bind(user.get())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(summary_from_row).collect()
}

/// Returns one group only if the user is a current member.
///
/// Used when accepting a selection callback: group IDs from callback data are
/// never trusted without a fresh membership check.
pub async fn get_for_user(
    pool: &SqlitePool,
    user: UserId,
    group: GroupId,
) -> Result<GroupSummary, GroupError> {
    let row: Option<SummaryRow> = sqlx::query_as(
        "SELECT g.id,
                g.name,
                gm.role,
                gm.color,
                gm.setup_completed_at,
                (SELECT COUNT(*) FROM group_members m WHERE m.group_id = g.id) AS member_count
         FROM group_members gm
         JOIN groups g ON g.id = gm.group_id
         WHERE gm.user_id = ? AND g.id = ?",
    )
    .bind(user.get())
    .bind(group.get())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => summary_from_row(row),
        None => Err(GroupError::NotAMember(group)),
    }
}

fn summary_from_row(row: SummaryRow) -> Result<GroupSummary, GroupError> {
    let id = GroupId::new(row.id).map_err(|_| GroupError::InvalidGroupId)?;
    let name =
        GroupName::parse(&row.name).map_err(|error| GroupError::CorruptData(error.to_string()))?;
    let role = MemberRole::from_db(&row.role)
        .map_err(|error| GroupError::CorruptData(error.to_string()))?;
    Ok(GroupSummary {
        id,
        name,
        role,
        color: row.color,
        member_count: row.member_count,
        setup_completed_at: row.setup_completed_at,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct OwnerRow {
    name: String,
    owner_user_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipTransfer {
    pub group_id: GroupId,
    pub group_name: String,
    pub new_owner: UserId,
}

/// Transfers ownership to an existing member and keeps the previous owner as a
/// regular member. Both role updates and the group record change atomically.
pub async fn transfer_ownership(
    pool: &SqlitePool,
    actor: UserId,
    group: GroupId,
    target: UserId,
) -> Result<OwnershipTransfer, GroupError> {
    if actor == target {
        return Err(GroupError::InvalidTransferTarget);
    }

    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let group_row: Option<OwnerRow> =
        sqlx::query_as("SELECT name, owner_user_id FROM groups WHERE id = ?")
            .bind(group.get())
            .fetch_optional(&mut *transaction)
            .await?;
    let Some(OwnerRow {
        name: group_name,
        owner_user_id: owner_raw,
    }) = group_row
    else {
        return Err(GroupError::GroupNotFound(group));
    };
    if owner_raw != actor.get() {
        return Err(GroupError::NotOwner(actor));
    }

    let target_membership: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
            .bind(group.get())
            .bind(target.get())
            .fetch_optional(&mut *transaction)
            .await?;
    if target_membership.is_none() {
        return Err(GroupError::TargetNotMember(target));
    }

    // Drop the old owner first so the partial unique owner index stays valid.
    sqlx::query("UPDATE group_members SET role = 'member' WHERE group_id = ? AND user_id = ?")
        .bind(group.get())
        .bind(actor.get())
        .execute(&mut *transaction)
        .await?;

    sqlx::query("UPDATE group_members SET role = 'owner' WHERE group_id = ? AND user_id = ?")
        .bind(group.get())
        .bind(target.get())
        .execute(&mut *transaction)
        .await?;

    sqlx::query("UPDATE groups SET owner_user_id = ?, updated_at = ? WHERE id = ?")
        .bind(target.get())
        .bind(super::now_millis())
        .bind(group.get())
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(OwnershipTransfer {
        group_id: group,
        group_name,
        new_owner: target,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedGroup {
    pub group_id: GroupId,
    pub group_name: String,
    /// Captured before deletion for best-effort notifications.
    pub member_telegram_ids: Vec<TelegramId>,
}

/// Deletes a group and all dependent data by cascade. Owner only.
pub async fn delete_group(
    pool: &SqlitePool,
    actor: UserId,
    group: GroupId,
) -> Result<DeletedGroup, GroupError> {
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let group_row: Option<OwnerRow> =
        sqlx::query_as("SELECT name, owner_user_id FROM groups WHERE id = ?")
            .bind(group.get())
            .fetch_optional(&mut *transaction)
            .await?;
    let Some(OwnerRow {
        name: group_name,
        owner_user_id: owner_raw,
    }) = group_row
    else {
        return Err(GroupError::GroupNotFound(group));
    };
    if owner_raw != actor.get() {
        return Err(GroupError::NotOwner(actor));
    }

    let raw_telegram_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT u.telegram_id
         FROM group_members m
         JOIN users u ON u.id = m.user_id
         WHERE m.group_id = ?",
    )
    .bind(group.get())
    .fetch_all(&mut *transaction)
    .await?;

    let mut member_telegram_ids = Vec::with_capacity(raw_telegram_ids.len());
    for raw in raw_telegram_ids {
        member_telegram_ids.push(
            TelegramId::new(raw).map_err(|error| GroupError::CorruptData(error.to_string()))?,
        );
    }

    sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(group.get())
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(DeletedGroup {
        group_id: group,
        group_name,
        member_telegram_ids,
    })
}

/// Resolves the current owner's Telegram ID for best-effort notifications.
pub async fn owner_telegram_id(
    pool: &SqlitePool,
    group: GroupId,
) -> Result<Option<TelegramId>, GroupError> {
    let raw: Option<i64> = sqlx::query_scalar(
        "SELECT u.telegram_id
         FROM groups g
         JOIN users u ON u.id = g.owner_user_id
         WHERE g.id = ?",
    )
    .bind(group.get())
    .fetch_optional(pool)
    .await?;

    match raw {
        Some(value) => {
            Ok(Some(TelegramId::new(value).map_err(|error| {
                GroupError::CorruptData(error.to_string())
            })?))
        }
        None => Ok(None),
    }
}
