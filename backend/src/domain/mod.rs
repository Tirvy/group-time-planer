//! Domain types and invariants that do not depend on Telegram, HTTP or SQLite.

pub mod group;
pub mod ids;
pub mod invitation;
pub mod member;
pub mod preference;
pub mod timezone;
pub mod weekday;

pub use group::{GroupName, GroupNameError, MAX_GROUP_MEMBERS, MAX_GROUP_NAME_CHARS};
pub use ids::{GroupId, IdError, InvitationId, TelegramId, UserId};
pub use invitation::{InvitationToken, InvitationTokenError};
pub use member::{MEMBER_COLORS, MemberRole, MemberRoleError, SetupState};
pub use preference::{MinuteOfDay, PreferenceError, PreferenceInterval, normalize_intervals};
pub use timezone::{IanaTimezone, TimezoneError};
pub use weekday::{Weekday, WeekdayError};
