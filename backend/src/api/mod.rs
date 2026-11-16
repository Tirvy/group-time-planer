pub mod auth;
mod error;
mod health;
mod me;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use sqlx::SqlitePool;

use crate::auth::AuthSettings;

pub use error::ApiError;

const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;

/// Shared application state. Domain services will hang off this struct in later
/// phases; handlers must never derive identity from client-supplied user IDs.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub auth: AuthSettings,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/auth/telegram", post(auth::telegram))
        .route("/api/me", get(me::get_me))
        .route("/api/me/timezone", put(me::update_timezone))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state)
}
