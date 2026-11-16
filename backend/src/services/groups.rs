use sqlx::{Connection, SqlitePool};

use crate::{
    db,
    domain::{GroupId, GroupName, MEMBER_COLORS, MemberRole, UserId},
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

#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("user {0} does not exist")]
    UserNotFound(UserId),
    #[error("database returned an invalid group id")]
    InvalidGroupId,
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
