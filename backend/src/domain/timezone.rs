use chrono_tz::Tz;

/// A validated IANA timezone identifier such as `Asia/Tbilisi`.
///
/// Fixed UTC offsets are intentionally not accepted: users must choose a named
/// zone so DST rules apply correctly to recurring preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IanaTimezone(Tz);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown IANA timezone: {0:?}")]
pub struct TimezoneError(pub String);

impl IanaTimezone {
    pub fn parse(value: &str) -> Result<Self, TimezoneError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(TimezoneError(value.to_owned()));
        }
        trimmed
            .parse::<Tz>()
            .map(Self)
            .map_err(|_| TimezoneError(value.to_owned()))
    }

    pub fn name(self) -> &'static str {
        self.0.name()
    }

    pub fn as_tz(self) -> Tz {
        self.0
    }
}

impl std::fmt::Display for IanaTimezone {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_iana_identifiers() {
        for name in ["Asia/Tbilisi", "Europe/Berlin", "America/New_York", "UTC"] {
            let timezone = IanaTimezone::parse(name).unwrap();
            assert_eq!(timezone.name(), name);
            assert_eq!(timezone.to_string(), name);
        }
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let timezone = IanaTimezone::parse("  Europe/Berlin  ").unwrap();
        assert_eq!(timezone.name(), "Europe/Berlin");
    }

    #[test]
    fn rejects_offsets_abbreviations_and_unknown_zones() {
        for name in ["+03:00", "MSK", "Mars/Olympus", "", "   "] {
            assert!(
                IanaTimezone::parse(name).is_err(),
                "{name:?} must be rejected"
            );
        }
    }
}
