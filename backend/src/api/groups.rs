use axum::{Json, extract::State};
use serde::Serialize;

use crate::{
    api::{AppState, error::ApiError},
    auth::extractor::AuthenticatedUser,
    services::groups,
};

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/types/generated/")]
pub struct GroupDto {
    /// Internal group ID serialized as a string.
    pub id: String,
    pub name: String,
    #[ts(type = "\"owner\" | \"member\"")]
    pub role: &'static str,
    pub color: String,
    pub member_count: i64,
    pub setup_completed: bool,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../frontend/src/types/generated/")]
pub struct GroupsResponse {
    pub groups: Vec<GroupDto>,
}

pub async fn list_groups(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<Json<GroupsResponse>, ApiError> {
    let groups = groups::list_for_user(&state.db, user.id)
        .await
        .map_err(|_| ApiError::internal())?;

    Ok(Json(GroupsResponse {
        groups: groups
            .into_iter()
            .map(|group| {
                let setup_completed = !group.setup_state().is_pending();
                GroupDto {
                    id: group.id.get().to_string(),
                    name: group.name.as_str().to_owned(),
                    role: group.role.as_db(),
                    color: group.color,
                    member_count: group.member_count,
                    setup_completed,
                }
            })
            .collect(),
    }))
}
