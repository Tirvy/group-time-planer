mod health;

use axum::{Router, routing::get};
use sqlx::SqlitePool;

/// Shared application state. Domain services will hang off this struct in later
/// phases; handlers must never derive identity from client-supplied user IDs.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .with_state(state)
}
