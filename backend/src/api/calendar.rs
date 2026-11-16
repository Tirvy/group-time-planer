use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::{
    api::{AppState, error::ApiError},
    auth::extractor::AuthenticatedUser,
    domain::{GroupId, IanaTimezone, UserId},
    services::{
        calendar::{self, CalendarError},
        groups::GroupError,
        members::MembersError,
        preferences::PreferencesError,
    },
};

#[derive(Debug, Serialize)]
pub struct GroupInfoDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ReferenceWeekDto {
    pub start_date: String,
    pub end_date: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct CalendarSegmentDto {
    pub day: u8,
    pub start_minute: u16,
    pub end_minute: u16,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct CalendarMemberDto {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub role: &'static str,
    pub color: String,
    pub setup_state: &'static str,
    pub segments: Vec<CalendarSegmentDto>,
}

#[derive(Debug, Serialize)]
pub struct GroupCalendarResponse {
    pub group: GroupInfoDto,
    pub viewer_timezone: String,
    pub reference_week: ReferenceWeekDto,
    pub members: Vec<CalendarMemberDto>,
}

#[derive(Debug, Serialize)]
pub struct MemberCalendarResponse {
    pub group: GroupInfoDto,
    pub viewer_timezone: String,
    pub reference_week: ReferenceWeekDto,
    pub member: CalendarMemberDto,
}

pub async fn get_group_calendar(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<i64>,
) -> Result<Json<GroupCalendarResponse>, ApiError> {
    let group = parse_group_id(group_id)?;
    let timezone = auth.require_timezone()?;

    let calendar =
        calendar::group_calendar(&state.db, auth.0.id, group, state.clock.now(), timezone)
            .await
            .map_err(map_calendar_error)?;

    Ok(Json(GroupCalendarResponse {
        group: GroupInfoDto {
            id: calendar.group_id.get().to_string(),
            name: calendar.group_name,
        },
        viewer_timezone: calendar.viewer_timezone.name().to_owned(),
        reference_week: reference_week_dto(
            calendar.week.start_date,
            calendar.week.end_date,
            calendar.week.start,
            calendar.week.end,
            timezone,
        ),
        members: calendar
            .members
            .into_iter()
            .map(|member| member_dto(member, timezone))
            .collect(),
    }))
}

pub async fn get_member_calendar(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((group_id, member_id)): Path<(i64, i64)>,
) -> Result<Json<MemberCalendarResponse>, ApiError> {
    let group = parse_group_id(group_id)?;
    let member = UserId::new(member_id).map_err(|_| ApiError::validation("invalid member id"))?;
    let timezone = auth.require_timezone()?;

    let calendar = calendar::member_calendar(
        &state.db,
        auth.0.id,
        group,
        member,
        state.clock.now(),
        timezone,
    )
    .await
    .map_err(map_calendar_error)?;

    let member = calendar
        .members
        .into_iter()
        .next()
        .ok_or_else(ApiError::internal)?;

    Ok(Json(MemberCalendarResponse {
        group: GroupInfoDto {
            id: calendar.group_id.get().to_string(),
            name: calendar.group_name,
        },
        viewer_timezone: calendar.viewer_timezone.name().to_owned(),
        reference_week: reference_week_dto(
            calendar.week.start_date,
            calendar.week.end_date,
            calendar.week.start,
            calendar.week.end,
            timezone,
        ),
        member: member_dto(member, timezone),
    }))
}

fn member_dto(member: calendar::CalendarMember, timezone: IanaTimezone) -> CalendarMemberDto {
    CalendarMemberDto {
        user_id: member.user_id.get().to_string(),
        first_name: member.first_name,
        last_name: member.last_name,
        username: member.username,
        role: member.role.as_db(),
        color: member.color,
        setup_state: member.setup_state.as_str(),
        segments: member
            .segments
            .into_iter()
            .map(|segment| CalendarSegmentDto {
                day: segment.day,
                start_minute: segment.start_minute,
                end_minute: segment.end_minute,
                start: format_local(segment.start, timezone),
                end: format_local(segment.end, timezone),
            })
            .collect(),
    }
}

pub(crate) fn reference_week_dto(
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    timezone: IanaTimezone,
) -> ReferenceWeekDto {
    ReferenceWeekDto {
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        start: format_local(start, timezone),
        end: format_local(end, timezone),
    }
}

pub(crate) fn format_local(datetime: DateTime<Utc>, timezone: IanaTimezone) -> String {
    datetime
        .with_timezone(&timezone.as_tz())
        .to_rfc3339_opts(SecondsFormat::Secs, false)
}

fn parse_group_id(raw: i64) -> Result<GroupId, ApiError> {
    GroupId::new(raw).map_err(|_| ApiError::validation("invalid group id"))
}

pub(crate) fn map_calendar_error(error: CalendarError) -> ApiError {
    match error {
        CalendarError::Group(GroupError::NotAMember(_)) => ApiError::not_a_member(),
        CalendarError::Group(GroupError::GroupNotFound(_)) => ApiError::group_not_found(),
        CalendarError::Group(_) => {
            tracing::error!("calendar group lookup failed");
            ApiError::internal()
        }
        CalendarError::Members(MembersError::NotAMember(_)) => ApiError::not_a_member(),
        CalendarError::Members(_) => {
            tracing::error!("calendar member lookup failed");
            ApiError::internal()
        }
        CalendarError::Preferences(PreferencesError::NotAMember(_)) => ApiError::not_a_member(),
        CalendarError::Preferences(_) => {
            tracing::error!("calendar preferences lookup failed");
            ApiError::internal()
        }
        CalendarError::MemberNotFound(_) => ApiError::member_not_found(),
        CalendarError::Time(_) | CalendarError::CorruptData(_) => {
            tracing::error!("calendar timezone conversion failed");
            ApiError::internal()
        }
    }
}
