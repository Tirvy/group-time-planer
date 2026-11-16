use sqlx::{Connection, SqlitePool};

use crate::{
    db,
    domain::{GroupId, GroupName, MEMBER_COLORS, MemberRole, SetupState, UserId},
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
    #[error("group {0} is not available to this user")]
    NotAMember(GroupId),
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
