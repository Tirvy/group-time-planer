/// Positive internal identifier or Telegram identifier.
///
/// SQLite `INTEGER PRIMARY KEY` values are positive; Telegram user IDs are
/// positive. Zero and negative values are rejected at every boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{kind} id must be positive, got {value}")]
pub struct IdError {
    pub kind: &'static str,
    pub value: i64,
}

macro_rules! define_id {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub fn new(value: i64) -> Result<Self, IdError> {
                if value > 0 {
                    Ok(Self(value))
                } else {
                    Err(IdError { kind: $kind, value })
                }
            }

            pub fn get(self) -> i64 {
                self.0
            }
        }

        impl TryFrom<i64> for $name {
            type Error = IdError;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

define_id!(UserId, "user");
define_id!(GroupId, "group");
define_id!(InvitationId, "invitation");
define_id!(TelegramId, "telegram");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_positive_ids() {
        assert!(UserId::new(0).is_err());
        assert!(GroupId::new(-5).is_err());
        assert!(TelegramId::new(-1).is_err());
    }

    #[test]
    fn accepts_positive_ids_and_round_trips() {
        let user = UserId::new(42).unwrap();
        assert_eq!(user.get(), 42);
        assert_eq!(i64::from(user), 42);
        assert_eq!(user.to_string(), "42");
        assert_eq!(UserId::try_from(7).unwrap().get(), 7);
    }

    #[test]
    fn id_types_are_distinct() {
        // Compile-time distinction is the point: these values cannot be mixed
        // accidentally because they are different types.
        let user = UserId::new(1).unwrap();
        let group = GroupId::new(1).unwrap();
        assert_eq!(user.get(), group.get());
    }
}
