use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    api::{
        ApiError, AppState,
        calendar::{GroupInfoDto, format_local, map_calendar_error, reference_week_dto},
    },
    auth::extractor::AuthenticatedUser,
    domain::{GroupId, IanaTimezone},
    services::{
        best_time::{self, RankedInterval},
        calendar::{self as calendar_service, GroupCalendar},
        timetable,
        timezone::TimeError,
    },
};

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/types/generated/")]
pub struct RankedIntervalDto {
    pub day: u8,
    pub start_minute: u16,
    pub end_minute: u16,
    pub start: String,
    pub end: String,
    pub count: usize,
    /// Members available for the whole range, in viewer order.
    pub members: Vec<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/types/generated/")]
pub struct BestTimeMemberDto {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub color: String,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/types/generated/")]
pub struct BestTimeResponse {
    pub group: GroupInfoDto,
    pub viewer_timezone: String,
    pub reference_week: super::calendar::ReferenceWeekDto,
    pub total_member_count: usize,
    pub configured_member_count: usize,
    pub pending_member_count: usize,
    /// Members referenced by `RankedIntervalDto::members`.
    pub members: Vec<BestTimeMemberDto>,
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

    let members: Vec<BestTimeMemberDto> = calendar
        .members
        .iter()
        .map(|member| BestTimeMemberDto {
            user_id: member.user_id.get().to_string(),
            first_name: member.first_name.clone(),
            last_name: member.last_name.clone(),
            username: member.username.clone(),
            color: member.color.clone(),
        })
        .collect();

    let ranked = best_time::rank(&calendar).map_err(map_time_error)?;
    let viewer_timezone = ranked.viewer_timezone;

    Ok(Json(BestTimeResponse {
        group: GroupInfoDto {
            id: ranked.group_id.get().to_string(),
            name: ranked.group_name,
        },
        viewer_timezone: viewer_timezone.name().to_owned(),
        reference_week: reference_week_dto(
            ranked.week.start_date,
            ranked.week.end_date,
            ranked.week.start,
            ranked.week.end,
            viewer_timezone,
        ),
        total_member_count: ranked.total_member_count,
        configured_member_count: ranked.configured_member_count,
        pending_member_count: ranked.pending_member_count,
        members,
        intervals: ranked
            .intervals
            .into_iter()
            .map(|interval| ranked_interval_dto(interval, viewer_timezone))
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

    let key = calendar_cache_key(&calendar);
    if let Some(cached) = state.png.get(&key) {
        return Ok(png_response(cached));
    }

    // Limit concurrent renders: each one allocates a multi-megabyte pixmap.
    let _permit = state.png.acquire().await?;

    // Another request may have filled the cache while this one waited.
    if let Some(cached) = state.png.get(&key) {
        return Ok(png_response(cached));
    }

    let now = state.clock.now();
    let png = tokio::task::spawn_blocking(move || timetable::render_png(&calendar, now))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(|error| {
            tracing::error!(error = %error, "timetable rendering failed");
            ApiError::internal()
        })?;

    let bytes = Bytes::from(png);
    state.png.put(key, bytes.clone());
    Ok(png_response(bytes))
}

fn png_response(bytes: Bytes) -> Response {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline; filename=\"timetable.png\""),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Content-derived cache key: identical calendar input reuses the same PNG.
fn calendar_cache_key(calendar: &GroupCalendar) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(calendar.group_id.get().to_le_bytes());
    hasher.update(calendar.group_name.as_bytes());
    hasher.update(calendar.viewer_timezone.name().as_bytes());
    hasher.update(calendar.week.start.timestamp().to_le_bytes());
    hasher.update(calendar.week.end.timestamp().to_le_bytes());

    for member in &calendar.members {
        hasher.update(member.user_id.get().to_le_bytes());
        hasher.update(member.first_name.as_deref().unwrap_or_default().as_bytes());
        hasher.update(member.last_name.as_deref().unwrap_or_default().as_bytes());
        hasher.update(member.username.as_deref().unwrap_or_default().as_bytes());
        hasher.update(member.role.as_db().as_bytes());
        hasher.update(member.color.as_bytes());
        hasher.update([u8::from(!member.setup_state.is_pending())]);
        for segment in &member.segments {
            hasher.update([segment.day]);
            hasher.update(segment.start_minute.to_le_bytes());
            hasher.update(segment.end_minute.to_le_bytes());
            hasher.update(segment.start.timestamp().to_le_bytes());
            hasher.update(segment.end.timestamp().to_le_bytes());
        }
    }

    hasher.finalize().into()
}

fn ranked_interval_dto(interval: RankedInterval, timezone: IanaTimezone) -> RankedIntervalDto {
    RankedIntervalDto {
        day: interval.day,
        start_minute: interval.start_minute,
        end_minute: interval.end_minute,
        start: format_local(interval.start, timezone),
        end: format_local(interval.end, timezone),
        count: interval.count(),
        members: interval
            .member_ids
            .into_iter()
            .map(|user_id| user_id.get().to_string())
            .collect(),
    }
}

fn parse_group_id(raw: i64) -> Result<GroupId, ApiError> {
    GroupId::new(raw).map_err(|_| ApiError::validation("invalid group id"))
}

fn map_time_error(error: TimeError) -> ApiError {
    tracing::error!(error = %error, "best-time conversion failed");
    ApiError::internal()
}
