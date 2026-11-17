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
    let row: UserRow = sqlx::query_as(
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

    row.into_user()
}

/// Resolves an existing user without writing.
///
/// The per-request authentication path uses this so that ordinary reads do not
/// take SQLite's single writer lock.
pub async fn find_by_telegram_id(
    pool: &SqlitePool,
    telegram_id: TelegramId,
) -> Result<Option<User>, UserError> {
    let row: Option<UserRow> = sqlx::query_as(
        "SELECT id, telegram_id, username, first_name, last_name, timezone
         FROM users
         WHERE telegram_id = ?",
    )
    .bind(telegram_id.get())
    .fetch_optional(pool)
    .await?;

    row.map(UserRow::into_user).transpose()
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
    let row: UserRow = sqlx::query_as(
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

    row.into_user()
}

/// Column order and types shared by every `users` query in this module.
#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: i64,
    telegram_id: i64,
    username: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    timezone: Option<String>,
}

impl UserRow {
    fn into_user(self) -> Result<User, UserError> {
        Ok(User {
            id: UserId::new(self.id).map_err(|_| UserError::InvalidUserId)?,
            telegram_id: TelegramId::new(self.telegram_id)
                .map_err(|_| UserError::InvalidTelegramId)?,
            username: self.username,
            first_name: self.first_name,
            last_name: self.last_name,
            timezone: self.timezone,
        })
    }
}
