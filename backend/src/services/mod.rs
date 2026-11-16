//! Application services shared by the Telegram bot and the Mini App API.
//!
//! Business rules live here, not in HTTP or Telegram handlers.

pub mod best_time;
pub mod calendar;
pub mod clock;
pub mod groups;
pub mod invitations;
pub mod members;
pub mod membership;
pub mod preferences;
pub mod timetable;
pub mod timezone;
pub mod users;

/// SQLite timestamps are UTC milliseconds since the Unix epoch.
pub(crate) fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
