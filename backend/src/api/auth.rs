use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{
    api::{AppState, error::ApiError, me::MeResponse},
    auth,
};

const MAX_INIT_DATA_BYTES: usize = 16 * 1024;

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    #[serde(rename = "initData")]
    pub init_data: String,
}

/// Validates Mini App `initData` and returns the authenticated user summary.
pub async fn telegram(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Json<MeResponse>, ApiError> {
    if body.init_data.len() > MAX_INIT_DATA_BYTES {
        return Err(ApiError::validation("initData is too large"));
    }

    let user = auth::authenticate(&state.db, &state.auth, &body.init_data)
        .await
        .map_err(ApiError::from_auth)?;

    Ok(Json(MeResponse { user: user.into() }))
}
