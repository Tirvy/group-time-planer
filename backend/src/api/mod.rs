pub mod auth;
mod calendar;
mod error;
mod groups;
mod health;
mod me;
mod preferences;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use sqlx::SqlitePool;
use teloxide::Bot;

use crate::{auth::AuthSettings, services::clock::Clock};

pub use error::ApiError;

const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;

/// Shared application state. Handlers must never derive identity from
/// client-supplied user IDs.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub auth: AuthSettings,
    /// Used only for best-effort owner notifications; absent in API-only mode.
    pub bot: Option<Bot>,
    /// Single injectable clock for deterministic reference-week calculations.
    pub clock: Arc<dyn Clock>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/auth/telegram", post(auth::telegram))
        .route("/api/me", get(me::get_me))
        .route("/api/me/timezone", put(me::update_timezone))
        .route("/api/groups", get(groups::list_groups))
        .route(
            "/api/groups/{group_id}/preferences",
            get(preferences::get_preferences).put(preferences::update_preferences),
        )
        .route(
            "/api/groups/{group_id}/calendar",
            get(calendar::get_group_calendar),
        )
        .route(
            "/api/groups/{group_id}/members/{member_id}/calendar",
            get(calendar::get_member_calendar),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state)
}
