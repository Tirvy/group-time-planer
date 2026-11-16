use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{AppState, error::ApiError},
    auth::extractor::AuthenticatedUser,
    bot::{notifications, texts},
    domain::{GroupId, PreferenceInterval, Weekday},
    services::{
        groups::{self, GroupError},
        preferences::{self, PreferencesError},
    },
};

const MAX_INTERVALS: usize = 336;

#[derive(Debug, Serialize, Deserialize)]
pub struct IntervalDto {
    pub weekday: u8,
    pub start_minute: u16,
    pub end_minute: u16,
}

#[derive(Debug, Serialize)]
pub struct PreferencesResponse {
    pub group_id: String,
    pub timezone: String,
    pub setup_completed: bool,
    pub intervals: Vec<IntervalDto>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub intervals: Vec<IntervalDto>,
}

pub async fn get_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<i64>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let group = group_id_from_path(group_id)?;
    let timezone = auth.require_timezone()?;

    let prefs = preferences::get_own(&state.db, auth.0.id, group)
        .await
        .map_err(map_preferences_error)?;

    Ok(Json(PreferencesResponse {
        group_id: group.get().to_string(),
        timezone: timezone.name().to_owned(),
        setup_completed: prefs.setup_completed_at.is_some(),
        intervals: prefs.intervals.into_iter().map(interval_dto).collect(),
    }))
}

/// Replaces the authenticated member's recurring local preferences.
pub async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(group_id): Path<i64>,
    Json(body): Json<UpdatePreferencesRequest>,
) -> Result<Json<PreferencesResponse>, ApiError> {
    let group = group_id_from_path(group_id)?;
    let timezone = auth.require_timezone()?;

    if body.intervals.len() > MAX_INTERVALS {
        return Err(ApiError::validation("too many intervals"));
    }
    let intervals = body
        .intervals
        .into_iter()
        .map(interval_from_dto)
        .collect::<Result<Vec<_>, _>>()?;

    let summary = groups::get_for_user(&state.db, auth.0.id, group)
        .await
        .map_err(map_group_error)?;

    let saved = preferences::replace(&state.db, auth.0.id, group, intervals)
        .await
        .map_err(map_preferences_error)?;

    if saved.changed
        && !summary.is_owner()
        && let Some(bot) = &state.bot
    {
        notifications::preferences_changed(
            bot,
            &state.db,
            group,
            summary.name.as_str(),
            &texts::display_name(&auth.0),
        )
        .await;
    }

    Ok(Json(PreferencesResponse {
        group_id: group.get().to_string(),
        timezone: timezone.name().to_owned(),
        setup_completed: saved.setup_completed_at.is_some(),
        intervals: saved.intervals.into_iter().map(interval_dto).collect(),
    }))
}

fn group_id_from_path(raw: i64) -> Result<GroupId, ApiError> {
    GroupId::new(raw).map_err(|_| ApiError::validation("invalid group id"))
}

fn interval_dto(interval: PreferenceInterval) -> IntervalDto {
    IntervalDto {
        weekday: interval.weekday().number(),
        start_minute: interval.start_minute(),
        end_minute: interval.end_minute(),
    }
}

fn interval_from_dto(dto: IntervalDto) -> Result<PreferenceInterval, ApiError> {
    let weekday =
        Weekday::from_number(dto.weekday).map_err(|_| ApiError::validation("invalid weekday"))?;
    PreferenceInterval::new(weekday, dto.start_minute, dto.end_minute)
        .map_err(|_| ApiError::validation("invalid preference interval"))
}

fn map_preferences_error(error: PreferencesError) -> ApiError {
    match error {
        PreferencesError::NotAMember(_) => ApiError::not_a_member(),
        PreferencesError::Validation(_) => ApiError::validation("invalid preference intervals"),
        PreferencesError::CorruptData(_) | PreferencesError::Database(_) => {
            tracing::error!("preferences storage error");
            ApiError::internal()
        }
    }
}

fn map_group_error(error: GroupError) -> ApiError {
    match error {
        GroupError::NotAMember(_) => ApiError::not_a_member(),
        GroupError::GroupNotFound(_) => ApiError::group_not_found(),
        _ => {
            tracing::error!("group lookup failed");
            ApiError::internal()
        }
    }
}
