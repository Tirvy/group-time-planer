use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::{
    domain::{GroupId, IanaTimezone, MemberRole, SetupState, UserId},
    services::{
        groups::{self, GroupError},
        members::{self, MemberSummary, MembersError},
        preferences::{self, PreferencesError},
        timezone::{self, ConvertedSegment, ReferenceWeek, TimeError},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarMember {
    pub user_id: UserId,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub role: MemberRole,
    pub color: String,
    pub setup_state: SetupState,
    pub segments: Vec<ConvertedSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupCalendar {
    pub group_id: GroupId,
    pub group_name: String,
    pub viewer_timezone: IanaTimezone,
    pub week: ReferenceWeek,
    pub members: Vec<CalendarMember>,
}

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error(transparent)]
    Group(#[from] GroupError),
    #[error(transparent)]
    Members(#[from] MembersError),
    #[error(transparent)]
    Preferences(#[from] PreferencesError),
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error("member {0} is not in this group")]
    MemberNotFound(UserId),
    #[error("stored calendar data is invalid: {0}")]
    CorruptData(String),
}

/// Builds the combined calendar for every member, converted into the viewer's
/// timezone for the viewer's reference week.
pub async fn group_calendar(
    pool: &SqlitePool,
    viewer: UserId,
    group: GroupId,
    now: DateTime<Utc>,
    viewer_timezone: IanaTimezone,
) -> Result<GroupCalendar, CalendarError> {
    let summary = groups::get_for_user(pool, viewer, group).await?;
    let week = timezone::reference_week(now, viewer_timezone)?;
    let members = members::list_for_group(pool, viewer, group).await?;

    let mut calendar_members = Vec::with_capacity(members.len());
    for member in members {
        calendar_members.push(build_member(pool, group, member, &week).await?);
    }

    Ok(GroupCalendar {
        group_id: group,
        group_name: summary.name.as_str().to_owned(),
        viewer_timezone,
        week,
        members: calendar_members,
    })
}

/// Builds a single member's converted calendar for every authorised viewer.
pub async fn member_calendar(
    pool: &SqlitePool,
    viewer: UserId,
    group: GroupId,
    target: UserId,
    now: DateTime<Utc>,
    viewer_timezone: IanaTimezone,
) -> Result<GroupCalendar, CalendarError> {
    let summary = groups::get_for_user(pool, viewer, group).await?;
    let week = timezone::reference_week(now, viewer_timezone)?;
    let members = members::list_for_group(pool, viewer, group).await?;

    let member = members
        .into_iter()
        .find(|member| member.user_id == target)
        .ok_or(CalendarError::MemberNotFound(target))?;

    let member = build_member(pool, group, member, &week).await?;

    Ok(GroupCalendar {
        group_id: group,
        group_name: summary.name.as_str().to_owned(),
        viewer_timezone,
        week,
        members: vec![member],
    })
}

async fn build_member(
    pool: &SqlitePool,
    group: GroupId,
    member: MemberSummary,
    week: &ReferenceWeek,
) -> Result<CalendarMember, CalendarError> {
    let preferences = preferences::get_for_member(pool, group, member.user_id).await?;
    let setup_state = preferences.setup_state();

    let segments = if setup_state.is_pending() || preferences.intervals.is_empty() {
        Vec::new()
    } else {
        let timezone = member
            .timezone
            .as_deref()
            .ok_or_else(|| CalendarError::CorruptData("member timezone is missing".to_owned()))?;
        let timezone = IanaTimezone::parse(timezone)
            .map_err(|error| CalendarError::CorruptData(error.to_string()))?;
        timezone::convert_schedule(&preferences.intervals, timezone, week)?
    };

    Ok(CalendarMember {
        user_id: member.user_id,
        first_name: member.first_name,
        last_name: member.last_name,
        username: member.username,
        role: member.role,
        color: member.color,
        setup_state,
        segments,
    })
}
