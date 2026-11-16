use sqlx::SqlitePool;

use crate::domain::{IanaTimezone, TelegramId, UserId};

/// Telegram profile data that is mutable display metadata, not identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramProfile {
    pub telegram_id: TelegramId,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub telegram_id: TelegramId,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// Raw stored IANA identifier; validated when the user configures it.
    pub timezone: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("user {0} does not exist")]
    NotFound(UserId),
    #[error("database returned an invalid user id")]
    InvalidUserId,
    #[error("database returned an invalid telegram id")]
    InvalidTelegramId,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl TelegramProfile {
    pub fn new(
        telegram_id: TelegramId,
        username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Self {
        Self {
            telegram_id,
            username: normalize(username),
            first_name: normalize(first_name),
            last_name: normalize(last_name),
        }
    }
}

fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Creates or refreshes the user row for a Telegram account.
///
/// Identity is `telegram_id`; usernames and display names are mutable metadata.
/// The selected timezone is intentionally not overwritten.
pub async fn upsert(pool: &SqlitePool, profile: &TelegramProfile) -> Result<User, UserError> {
    let now = super::now_millis();
    let row: (
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as(
        "INSERT INTO users
             (telegram_id, username, first_name, last_name, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (telegram_id) DO UPDATE SET
             username = excluded.username,
             first_name = excluded.first_name,
             last_name = excluded.last_name,
             updated_at = excluded.updated_at
         RETURNING id, telegram_id, username, first_name, last_name, timezone",
    )
    .bind(profile.telegram_id.get())
    .bind(profile.username.as_deref())
    .bind(profile.first_name.as_deref())
    .bind(profile.last_name.as_deref())
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    user_from_row(row)
}

/// Persists the user's explicit IANA timezone choice.
///
/// The same value governs how their recurring preferences are interpreted and
/// how schedules are displayed to them.
pub async fn update_timezone(
    pool: &SqlitePool,
    user_id: UserId,
    timezone: IanaTimezone,
) -> Result<User, UserError> {
    let row: (
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = sqlx::query_as(
        "UPDATE users
         SET timezone = ?, updated_at = ?
         WHERE id = ?
         RETURNING id, telegram_id, username, first_name, last_name, timezone",
    )
    .bind(timezone.name())
    .bind(super::now_millis())
    .bind(user_id.get())
    .fetch_optional(pool)
    .await?
    .ok_or(UserError::NotFound(user_id))?;

    user_from_row(row)
}

type UserRow = (
    i64,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn user_from_row(
    (id, telegram_id, username, first_name, last_name, timezone): UserRow,
) -> Result<User, UserError> {
    Ok(User {
        id: UserId::new(id).map_err(|_| UserError::InvalidUserId)?,
        telegram_id: TelegramId::new(telegram_id).map_err(|_| UserError::InvalidTelegramId)?,
        username,
        first_name,
        last_name,
        timezone,
    })
}
