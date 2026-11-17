//! Telegram Mini App authentication.
//!
//! Identity is established only by validating Telegram `initData` server-side
//! with the bot token. Client-supplied user IDs are never trusted.

pub mod extractor;
pub mod init_data;

use std::time::Duration;

use sqlx::SqlitePool;

use crate::{
    config::SecretString,
    services::users::{self, TelegramProfile, User},
};

pub use init_data::{InitDataError, TelegramIdentity};

/// Authentication configuration shared by the API state.
#[derive(Clone)]
pub struct AuthSettings {
    pub bot_token: Option<SecretString>,
    pub max_age: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthUserError {
    #[error("Mini App authentication is not configured")]
    Unavailable,
    #[error(transparent)]
    InitData(#[from] InitDataError),
    #[error(transparent)]
    User(#[from] users::UserError),
}

/// Validates `initData`, resolves the Telegram identity and creates or refreshes
/// the user row. Runs on the explicit login endpoint (`POST /api/auth/telegram`)
/// so the profile metadata is refreshed once per Mini App open.
pub async fn login(
    pool: &SqlitePool,
    settings: &AuthSettings,
    init_data: &str,
) -> Result<User, AuthUserError> {
    let identity = verify(settings, init_data)?;
    let profile = profile_from(identity);

    Ok(users::upsert(pool, &profile).await?)
}

/// Per-request authentication for every protected route.
///
/// Validates `initData` and reads the existing user row. It deliberately does
/// not refresh the profile: reads must not take SQLite's single writer lock.
/// A row is created only if the user has never been seen before, which keeps
/// header-only clients working.
pub async fn authenticate(
    pool: &SqlitePool,
    settings: &AuthSettings,
    init_data: &str,
) -> Result<User, AuthUserError> {
    let identity = verify(settings, init_data)?;
    if let Some(user) = users::find_by_telegram_id(pool, identity.telegram_id).await? {
        return Ok(user);
    }

    let profile = profile_from(identity);
    Ok(users::upsert(pool, &profile).await?)
}

fn verify(
    settings: &AuthSettings,
    init_data: &str,
) -> Result<init_data::TelegramIdentity, AuthUserError> {
    let token = settings
        .bot_token
        .as_ref()
        .ok_or(AuthUserError::Unavailable)?;

    let now_secs = chrono::Utc::now().timestamp();
    Ok(init_data::validate(
        init_data,
        token.expose(),
        now_secs,
        settings.max_age.as_secs() as i64,
    )?)
}

fn profile_from(identity: init_data::TelegramIdentity) -> TelegramProfile {
    TelegramProfile::new(
        identity.telegram_id,
        identity.username,
        identity.first_name,
        identity.last_name,
    )
}
