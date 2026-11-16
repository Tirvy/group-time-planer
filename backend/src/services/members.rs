use sqlx::SqlitePool;

use crate::domain::{GroupId, MemberRole, SetupState, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberSummary {
    pub user_id: UserId,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub role: MemberRole,
    pub color: String,
    pub timezone: Option<String>,
    pub setup_completed_at: Option<i64>,
    pub joined_at: i64,
}

impl MemberSummary {
    pub fn setup_state(&self) -> SetupState {
        SetupState::from_completed_at(self.setup_completed_at)
    }

    pub fn is_owner(&self) -> bool {
        self.role.is_owner()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MembersError {
    #[error("group {0} is not available to this user")]
    NotAMember(GroupId),
    #[error("stored member data is invalid: {0}")]
    CorruptData(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Lists members of a group after verifying that the viewer belongs to it.
pub async fn list_for_group(
    pool: &SqlitePool,
    viewer: UserId,
    group: GroupId,
) -> Result<Vec<MemberSummary>, MembersError> {
    ensure_membership(pool, viewer, group).await?;
    list_unchecked(pool, group).await
}

/// Lists members without an authorization check.
///
/// Only call after authorization has been established, for example by the
/// invitation preview flow.
pub async fn list_unchecked(
    pool: &SqlitePool,
    group: GroupId,
) -> Result<Vec<MemberSummary>, MembersError> {
    type MemberRow = (
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<String>,
        Option<i64>,
        i64,
    );
    let rows: Vec<MemberRow> = sqlx::query_as(
        "SELECT u.id,
                u.first_name,
                u.last_name,
                u.username,
                gm.role,
                gm.color,
                u.timezone,
                gm.setup_completed_at,
                gm.joined_at
         FROM group_members gm
         JOIN users u ON u.id = gm.user_id
         WHERE gm.group_id = ?
         ORDER BY CASE gm.role WHEN 'owner' THEN 0 ELSE 1 END,
                  gm.joined_at ASC,
                  u.id ASC",
    )
    .bind(group.get())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(
            |(
                user_id,
                first_name,
                last_name,
                username,
                role,
                color,
                timezone,
                setup_completed_at,
                joined_at,
            )| {
                Ok(MemberSummary {
                    user_id: UserId::new(user_id)
                        .map_err(|error| MembersError::CorruptData(error.to_string()))?,
                    first_name,
                    last_name,
                    username,
                    role: MemberRole::from_db(&role)
                        .map_err(|error| MembersError::CorruptData(error.to_string()))?,
                    color,
                    timezone,
                    setup_completed_at,
                    joined_at,
                })
            },
        )
        .collect()
}

pub async fn ensure_membership(
    pool: &SqlitePool,
    viewer: UserId,
    group: GroupId,
) -> Result<(), MembersError> {
    let membership: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM group_members WHERE group_id = ? AND user_id = ?")
            .bind(group.get())
            .bind(viewer.get())
            .fetch_optional(pool)
            .await?;

    if membership.is_none() {
        return Err(MembersError::NotAMember(group));
    }
    Ok(())
}
