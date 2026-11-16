use sqlx::{Connection, SqlitePool};

use crate::{
    db,
    domain::{GroupId, MAX_GROUP_MEMBERS, MEMBER_COLORS, MemberRole, SetupState, UserId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    pub group_id: GroupId,
    pub user_id: UserId,
    pub role: MemberRole,
    pub color: String,
    pub setup_completed_at: Option<i64>,
    pub joined_at: i64,
}

impl Membership {
    pub fn setup_state(&self) -> SetupState {
        SetupState::from_completed_at(self.setup_completed_at)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MembershipError {
    #[error("group {0} does not exist")]
    GroupNotFound(GroupId),
    #[error("user {0} does not exist")]
    UserNotFound(UserId),
    #[error("user {user_id} is already a member of group {group_id}")]
    AlreadyMember { group_id: GroupId, user_id: UserId },
    #[error("group {group_id} already has {max} members")]
    GroupFull { group_id: GroupId, max: usize },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Adds a regular member to a group inside one write transaction.
///
/// Capacity is re-checked under the write lock and is also enforced by a
/// database trigger, so concurrent joins cannot exceed the cap. The member gets
/// the first unused color from the group palette.
pub async fn add(
    pool: &SqlitePool,
    group_id: GroupId,
    user_id: UserId,
) -> Result<Membership, MembershipError> {
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin_with(db::BEGIN_IMMEDIATE).await?;

    let group_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM groups WHERE id = ?")
        .bind(group_id.get())
        .fetch_optional(&mut *transaction)
        .await?;
    if group_exists.is_none() {
        return Err(MembershipError::GroupNotFound(group_id));
    }

    let user_exists: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
        .bind(user_id.get())
        .fetch_optional(&mut *transaction)
        .await?;
    if user_exists.is_none() {
        return Err(MembershipError::UserNotFound(user_id));
    }

    let existing_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM group_members WHERE group_id = ? AND user_id = ?")
            .bind(group_id.get())
            .bind(user_id.get())
            .fetch_optional(&mut *transaction)
            .await?;
    if existing_role.is_some() {
        return Err(MembershipError::AlreadyMember { group_id, user_id });
    }

    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE group_id = ?")
            .bind(group_id.get())
            .fetch_one(&mut *transaction)
            .await?;
    if member_count >= MAX_GROUP_MEMBERS as i64 {
        return Err(MembershipError::GroupFull {
            group_id,
            max: MAX_GROUP_MEMBERS,
        });
    }

    let used_colors: Vec<String> =
        sqlx::query_scalar("SELECT color FROM group_members WHERE group_id = ?")
            .bind(group_id.get())
            .fetch_all(&mut *transaction)
            .await?;
    let color = MEMBER_COLORS
        .iter()
        .copied()
        .find(|candidate| {
            !used_colors
                .iter()
                .any(|used| used.eq_ignore_ascii_case(candidate))
        })
        .ok_or(MembershipError::GroupFull {
            group_id,
            max: MAX_GROUP_MEMBERS,
        })?;

    let now = super::now_millis();
    sqlx::query(
        "INSERT INTO group_members
             (group_id, user_id, role, color, setup_completed_at, joined_at)
         VALUES (?, ?, ?, ?, NULL, ?)",
    )
    .bind(group_id.get())
    .bind(user_id.get())
    .bind(MemberRole::Member.as_db())
    .bind(color)
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(Membership {
        group_id,
        user_id,
        role: MemberRole::Member,
        color: color.to_owned(),
        setup_completed_at: None,
        joined_at: now,
    })
}
