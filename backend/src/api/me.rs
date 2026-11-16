use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    api::{AppState, error::ApiError},
    auth::extractor::AuthenticatedUser,
    domain::IanaTimezone,
    services::users::{self, User},
};

#[derive(Debug, Serialize)]
pub struct UserDto {
    /// Telegram IDs are strings to avoid JavaScript precision loss.
    pub telegram_id: String,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub timezone: Option<String>,
    pub timezone_set: bool,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        let timezone_set = user.timezone.is_some();
        Self {
            telegram_id: user.telegram_id.get().to_string(),
            username: user.username,
            first_name: user.first_name,
            last_name: user.last_name,
            timezone: user.timezone,
            timezone_set,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserDto,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTimezoneRequest {
    pub timezone: String,
}

pub async fn get_me(AuthenticatedUser(user): AuthenticatedUser) -> Json<MeResponse> {
    Json(MeResponse {
        user: UserDto::from(user),
    })
}

/// Persists the explicit IANA timezone choice. The same value is later used to
/// interpret preferences and to display schedules.
pub async fn update_timezone(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(body): Json<UpdateTimezoneRequest>,
) -> Result<Json<MeResponse>, ApiError> {
    let timezone = IanaTimezone::parse(&body.timezone).map_err(|_| ApiError::timezone_invalid())?;

    let updated = users::update_timezone(&state.db, user.id, timezone)
        .await
        .map_err(|_| ApiError::internal())?;

    Ok(Json(MeResponse {
        user: UserDto::from(updated),
    }))
}
