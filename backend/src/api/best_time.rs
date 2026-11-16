use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use serde::Serialize;

use crate::{
    api::{
        ApiError, AppState,
        calendar::{GroupInfoDto, format_local, map_calendar_error, reference_week_dto},
    },
    auth::extractor::AuthenticatedUser,
    domain::{GroupId, IanaTimezone},
    services::{
        best_time::{self, RankedInterval},
        calendar as calendar_service, timetable,
        timezone::TimeError,
    },
};

#[derive(Debug, Serialize)]
pub struct RankedIntervalDto {
    pub day: u8,
    pub start_minute: u16,
    pub end_minute: u16,
    pub start: String,
    pub end: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct BestTimeResponse {
    pub group: GroupInfoDto,
    pub viewer_timezone: String,
    pub reference_week: super::calendar::ReferenceWeekDto,
    pub total_member_count: usize,
    pub configured_member_count: usize,
    pub pending_member_count: usize,
    pub intervals: Vec<RankedIntervalDto>,
}

pub async fn get_best_time(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<i64>,
) -> Result<Json<BestTimeResponse>, ApiError> {
    let group = parse_group_id(group_id)?;
    let timezone = auth.require_timezone()?;

    let calendar =
        calendar_service::group_calendar(&state.db, auth.0.id, group, state.clock.now(), timezone)
            .await
            .map_err(map_calendar_error)?;

    let ranked = best_time::rank(&calendar).map_err(map_time_error)?;

    Ok(Json(BestTimeResponse {
        group: GroupInfoDto {
            id: ranked.group_id.get().to_string(),
            name: ranked.group_name,
        },
        viewer_timezone: ranked.viewer_timezone.name().to_owned(),
        reference_week: reference_week_dto(
            ranked.week.start_date,
            ranked.week.end_date,
            ranked.week.start,
            ranked.week.end,
            ranked.viewer_timezone,
        ),
        total_member_count: ranked.total_member_count,
        configured_member_count: ranked.configured_member_count,
        pending_member_count: ranked.pending_member_count,
        intervals: ranked
            .intervals
            .into_iter()
            .map(|interval| ranked_interval_dto(interval, ranked.viewer_timezone))
            .collect(),
    }))
}

pub async fn get_timetable_png(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<i64>,
) -> Result<Response, ApiError> {
    let group = parse_group_id(group_id)?;
    let timezone = auth.require_timezone()?;

    let calendar =
        calendar_service::group_calendar(&state.db, auth.0.id, group, state.clock.now(), timezone)
            .await
            .map_err(map_calendar_error)?;

    let now = state.clock.now();
    let png = tokio::task::spawn_blocking(move || timetable::render_png(&calendar, now))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(|error| {
            tracing::error!(error = %error, "timetable rendering failed");
            ApiError::internal()
        })?;

    let mut response = Response::new(Body::from(png));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline; filename=\"timetable.png\""),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

fn ranked_interval_dto(interval: RankedInterval, timezone: IanaTimezone) -> RankedIntervalDto {
    RankedIntervalDto {
        day: interval.day,
        start_minute: interval.start_minute,
        end_minute: interval.end_minute,
        start: format_local(interval.start, timezone),
        end: format_local(interval.end, timezone),
        count: interval.count,
    }
}

fn parse_group_id(raw: i64) -> Result<GroupId, ApiError> {
    GroupId::new(raw).map_err(|_| ApiError::validation("invalid group id"))
}

fn map_time_error(error: TimeError) -> ApiError {
    tracing::error!(error = %error, "best-time conversion failed");
    ApiError::internal()
}
