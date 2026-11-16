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
/// the user row.
pub async fn authenticate(
    pool: &SqlitePool,
    settings: &AuthSettings,
    init_data: &str,
) -> Result<User, AuthUserError> {
    let token = settings
        .bot_token
        .as_ref()
        .ok_or(AuthUserError::Unavailable)?;

    let now_secs = chrono::Utc::now().timestamp();
    let identity = init_data::validate(
        init_data,
        token.expose(),
        now_secs,
        settings.max_age.as_secs() as i64,
    )?;

    let profile = TelegramProfile::new(
        identity.telegram_id,
        identity.username,
        identity.first_name,
        identity.last_name,
    );

    Ok(users::upsert(pool, &profile).await?)
}
