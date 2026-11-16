/// Maximum group size, including the owner.
pub const MAX_GROUP_MEMBERS: usize = 20;

/// Maximum group name length in Unicode characters (not bytes).
pub const MAX_GROUP_NAME_CHARS: usize = 64;

/// Trimmed, validated group display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupName(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GroupNameError {
    #[error("group name must not be empty")]
    Empty,
    #[error("group name must be at most {max} characters, got {actual}")]
    TooLong { max: usize, actual: usize },
    #[error("group name must not contain control characters")]
    ControlCharacter,
}

impl GroupName {
    /// Trims the input, then enforces a 1..=64 character length and rejects
    /// control characters that would break bot messages or the UI.
    pub fn parse(raw: &str) -> Result<Self, GroupNameError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(GroupNameError::Empty);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(GroupNameError::ControlCharacter);
        }
        let actual = trimmed.chars().count();
        if actual > MAX_GROUP_NAME_CHARS {
            return Err(GroupNameError::TooLong {
                max: MAX_GROUP_NAME_CHARS,
                actual,
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GroupName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_unicode_names() {
        let name = GroupName::parse("  Мифик+ по средам  ").unwrap();
        assert_eq!(name.as_str(), "Мифик+ по средам");
    }

    #[test]
    fn rejects_empty_or_whitespace_only_names() {
        assert_eq!(GroupName::parse(""), Err(GroupNameError::Empty));
        assert_eq!(GroupName::parse("   \t\n "), Err(GroupNameError::Empty));
    }

    #[test]
    fn counts_characters_not_bytes() {
        let exactly_64 = "я".repeat(MAX_GROUP_NAME_CHARS);
        assert!(GroupName::parse(&exactly_64).is_ok());

        let too_long = "я".repeat(MAX_GROUP_NAME_CHARS + 1);
        assert_eq!(
            GroupName::parse(&too_long),
            Err(GroupNameError::TooLong {
                max: MAX_GROUP_NAME_CHARS,
                actual: MAX_GROUP_NAME_CHARS + 1,
            })
        );
    }

    #[test]
    fn rejects_control_characters() {
        assert_eq!(
            GroupName::parse("Рейд\nвторая строка"),
            Err(GroupNameError::ControlCharacter)
        );
    }
}
