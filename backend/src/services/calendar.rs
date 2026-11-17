use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::{
    domain::{GroupId, IanaTimezone, MemberRole, SetupState, UserId},
    services::{
        groups::{self, GroupError},
        members::{self, MembersError},
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
    /// The member's own IANA timezone, when configured.
    pub timezone: Option<String>,
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
    Time(#[from] TimeError),
    #[error("member {0} is not in this group")]
    MemberNotFound(UserId),
    #[error("stored calendar data is invalid: {0}")]
    CorruptData(String),
}

/// Builds the combined calendar for every member, converted into the viewer's
/// timezone for the viewer's reference week.
///
/// Uses two queries regardless of group size: the membership-checked group
/// summary and one member/preference join.
pub async fn group_calendar(
    pool: &SqlitePool,
    viewer: UserId,
    group: GroupId,
    now: DateTime<Utc>,
    viewer_timezone: IanaTimezone,
) -> Result<GroupCalendar, CalendarError> {
    let summary = groups::get_for_user(pool, viewer, group).await?;
    let week = timezone::reference_week(now, viewer_timezone)?;
    let entries = members::list_with_intervals_unchecked(pool, group).await?;

    let mut calendar_members = Vec::with_capacity(entries.len());
    for entry in entries {
        calendar_members.push(build_member(entry, &week)?);
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
    let entries = members::list_with_intervals_unchecked(pool, group).await?;

    let entry = entries
        .into_iter()
        .find(|entry| entry.member.user_id == target)
        .ok_or(CalendarError::MemberNotFound(target))?;

    let member = build_member(entry, &week)?;

    Ok(GroupCalendar {
        group_id: group,
        group_name: summary.name.as_str().to_owned(),
        viewer_timezone,
        week,
        members: vec![member],
    })
}

fn build_member(
    entry: members::MemberWithIntervals,
    week: &ReferenceWeek,
) -> Result<CalendarMember, CalendarError> {
    let setup_state = entry.member.setup_state();

    let segments = if setup_state.is_pending() || entry.intervals.is_empty() {
        Vec::new()
    } else {
        let timezone =
            entry.member.timezone.as_deref().ok_or_else(|| {
                CalendarError::CorruptData("member timezone is missing".to_owned())
            })?;
        let timezone = IanaTimezone::parse(timezone)
            .map_err(|error| CalendarError::CorruptData(error.to_string()))?;
        timezone::convert_schedule(&entry.intervals, timezone, week)?
    };

    Ok(CalendarMember {
        user_id: entry.member.user_id,
        first_name: entry.member.first_name,
        last_name: entry.member.last_name,
        username: entry.member.username,
        role: entry.member.role,
        color: entry.member.color,
        timezone: entry.member.timezone,
        setup_state,
        segments,
    })
}
