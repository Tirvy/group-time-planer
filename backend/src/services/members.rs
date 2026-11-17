use sqlx::SqlitePool;

use crate::domain::{
    GroupId, MemberRole, PreferenceInterval, SetupState, UserId, Weekday, normalize_intervals,
};

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

/// A group member with their stored recurring intervals.
///
/// Loaded in one query for every member so calendar, best-time and PNG views do
/// not issue per-member queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberWithIntervals {
    pub member: MemberSummary,
    pub intervals: Vec<PreferenceInterval>,
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

    rows.into_iter().map(MemberRow::into_summary).collect()
}

/// Lists every member with their stored intervals in a single query.
///
/// Members without preferences still appear (via `LEFT JOIN`) with an empty
/// interval list. Callers are responsible for the membership check.
pub async fn list_with_intervals_unchecked(
    pool: &SqlitePool,
    group: GroupId,
) -> Result<Vec<MemberWithIntervals>, MembersError> {
    let rows: Vec<MemberIntervalRow> = sqlx::query_as(
        "SELECT u.id,
                u.first_name,
                u.last_name,
                u.username,
                gm.role,
                gm.color,
                u.timezone,
                gm.setup_completed_at,
                gm.joined_at,
                p.weekday,
                p.start_minute,
                p.end_minute
         FROM group_members gm
         JOIN users u ON u.id = gm.user_id
         LEFT JOIN weekly_preferences p
                ON p.group_id = gm.group_id AND p.user_id = gm.user_id
         WHERE gm.group_id = ?
         ORDER BY CASE gm.role WHEN 'owner' THEN 0 ELSE 1 END,
                  gm.joined_at ASC,
                  u.id ASC,
                  p.weekday ASC,
                  p.start_minute ASC",
    )
    .bind(group.get())
    .fetch_all(pool)
    .await?;

    let mut members: Vec<MemberWithIntervals> = Vec::new();
    for row in rows {
        let starts_new_member = members
            .last()
            .is_none_or(|entry| entry.member.user_id.get() != row.id);
        if starts_new_member {
            members.push(MemberWithIntervals {
                member: row.summary()?,
                intervals: Vec::new(),
            });
        }

        let (Some(weekday), Some(start), Some(end)) =
            (row.weekday, row.start_minute, row.end_minute)
        else {
            continue;
        };

        let interval = parse_interval(weekday, start, end)?;
        if let Some(entry) = members.last_mut() {
            entry.intervals.push(interval);
        }
    }

    for entry in &mut members {
        entry.intervals = normalize_intervals(std::mem::take(&mut entry.intervals))
            .map_err(|error| MembersError::CorruptData(error.to_string()))?;
    }

    Ok(members)
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

fn parse_interval(
    weekday: i64,
    start_minute: i64,
    end_minute: i64,
) -> Result<PreferenceInterval, MembersError> {
    let weekday =
        Weekday::from_db(weekday).map_err(|error| MembersError::CorruptData(error.to_string()))?;
    let start = u16::try_from(start_minute)
        .map_err(|_| MembersError::CorruptData("start_minute out of range".to_owned()))?;
    let end = u16::try_from(end_minute)
        .map_err(|_| MembersError::CorruptData("end_minute out of range".to_owned()))?;
    PreferenceInterval::new(weekday, start, end)
        .map_err(|error| MembersError::CorruptData(error.to_string()))
}

/// Shared member columns, selected by both member queries.
#[derive(Debug, sqlx::FromRow)]
struct MemberRow {
    id: i64,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    role: String,
    color: String,
    timezone: Option<String>,
    setup_completed_at: Option<i64>,
    joined_at: i64,
}

impl MemberRow {
    fn into_summary(self) -> Result<MemberSummary, MembersError> {
        Ok(MemberSummary {
            user_id: UserId::new(self.id)
                .map_err(|error| MembersError::CorruptData(error.to_string()))?,
            first_name: self.first_name,
            last_name: self.last_name,
            username: self.username,
            role: MemberRole::from_db(&self.role)
                .map_err(|error| MembersError::CorruptData(error.to_string()))?,
            color: self.color,
            timezone: self.timezone,
            setup_completed_at: self.setup_completed_at,
            joined_at: self.joined_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MemberIntervalRow {
    id: i64,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    role: String,
    color: String,
    timezone: Option<String>,
    setup_completed_at: Option<i64>,
    joined_at: i64,
    weekday: Option<i64>,
    start_minute: Option<i64>,
    end_minute: Option<i64>,
}

impl MemberIntervalRow {
    fn summary(&self) -> Result<MemberSummary, MembersError> {
        MemberRow {
            id: self.id,
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            username: self.username.clone(),
            role: self.role.clone(),
            color: self.color.clone(),
            timezone: self.timezone.clone(),
            setup_completed_at: self.setup_completed_at,
            joined_at: self.joined_at,
        }
        .into_summary()
    }
}
