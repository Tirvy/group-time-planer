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
pub async fn list_for_user(
    pool: &SqlitePool,
    user: UserId,
) -> Result<Vec<GroupSummary>, GroupError> {
    let rows: Vec<(i64, String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT g.id,
                g.name,
                gm.role,
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
    let row: Option<(i64, String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT g.id,
                g.name,
                gm.role,
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

fn summary_from_row(
    (id, name, role, setup_completed_at, member_count): (i64, String, String, Option<i64>, i64),
) -> Result<GroupSummary, GroupError> {
    let id = GroupId::new(id).map_err(|_| GroupError::InvalidGroupId)?;
    let name =
        GroupName::parse(&name).map_err(|error| GroupError::CorruptData(error.to_string()))?;
    let role =
        MemberRole::from_db(&role).map_err(|error| GroupError::CorruptData(error.to_string()))?;
    Ok(GroupSummary {
        id,
        name,
        role,
        member_count,
        setup_completed_at,
    })
}

/// Renames a group. Owner only.
pub async fn rename(
    pool: &SqlitePool,
    actor: UserId,
    group: GroupId,
    name: &GroupName,
) -> Result<(), GroupError> {
    let owner: Option<i64> = sqlx::query_scalar("SELECT owner_user_id FROM groups WHERE id = ?")
        .bind(group.get())
        .fetch_optional(pool)
        .await?;
    let Some(owner) = owner else {
        return Err(GroupError::GroupNotFound(group));
    };
    if owner != actor.get() {
        return Err(GroupError::NotOwner(actor));
    }

    sqlx::query("UPDATE groups SET name = ?, updated_at = ? WHERE id = ?")
        .bind(name.as_str())
        .bind(super::now_millis())
        .bind(group.get())
        .execute(pool)
        .await?;

    Ok(())
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

    let group_row: Option<(String, i64)> =
        sqlx::query_as("SELECT name, owner_user_id FROM groups WHERE id = ?")
            .bind(group.get())
            .fetch_optional(&mut *transaction)
            .await?;
    let Some((group_name, owner_raw)) = group_row else {
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

    let group_row: Option<(String, i64)> =
        sqlx::query_as("SELECT name, owner_user_id FROM groups WHERE id = ?")
            .bind(group.get())
            .fetch_optional(&mut *transaction)
            .await?;
    let Some((group_name, owner_raw)) = group_row else {
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
