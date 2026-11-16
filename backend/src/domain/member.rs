/// Stable member colors within a group. Assignment is deterministic by palette
/// order; see `services::membership`.
///
/// The exact palette remains an open product question (docs/09), so only the
/// stored per-membership color is treated as stable.
pub const MEMBER_COLORS: [&str; 20] = [
    "#4DD0E1", "#7E57C2", "#66BB6A", "#FFA726", "#EF5350", "#42A5F5", "#EC407A", "#26A69A",
    "#AB47BC", "#9CCC65", "#FF7043", "#29B6F6", "#D4E157", "#8D6E63", "#B0BEC5", "#FFCA28",
    "#5C6BC0", "#26C6DA", "#C0CA33", "#F06292",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRole {
    Owner,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown member role: {0:?}")]
pub struct MemberRoleError(pub String);

impl MemberRole {
    pub fn as_db(self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Member => "member",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, MemberRoleError> {
        match value {
            "owner" => Ok(MemberRole::Owner),
            "member" => Ok(MemberRole::Member),
            other => Err(MemberRoleError(other.to_owned())),
        }
    }

    pub fn is_owner(self) -> bool {
        matches!(self, MemberRole::Owner)
    }
}

/// `setup_completed_at IS NULL` means pending. A completed member may have zero
/// preferred intervals; that is materially different from pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupState {
    Pending,
    Completed,
}

impl SetupState {
    pub fn from_completed_at(completed_at: Option<i64>) -> Self {
        if completed_at.is_some() {
            SetupState::Completed
        } else {
            SetupState::Pending
        }
    }

    pub fn is_pending(self) -> bool {
        matches!(self, SetupState::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_contains_twenty_unique_colors() {
        assert_eq!(MEMBER_COLORS.len(), 20);
        let mut colors = MEMBER_COLORS.to_vec();
        colors.sort_unstable();
        colors.dedup();
        assert_eq!(colors.len(), MEMBER_COLORS.len());
        for color in MEMBER_COLORS {
            assert_eq!(color.len(), 7);
            assert!(color.starts_with('#'));
            assert!(color[1..].chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn role_round_trips_through_database_values() {
        for role in [MemberRole::Owner, MemberRole::Member] {
            assert_eq!(MemberRole::from_db(role.as_db()).unwrap(), role);
        }
        assert!(MemberRole::from_db("admin").is_err());
    }

    #[test]
    fn setup_state_distinguishes_pending_from_completed_empty() {
        assert_eq!(SetupState::from_completed_at(None), SetupState::Pending);
        assert!(SetupState::Pending.is_pending());
        assert_eq!(
            SetupState::from_completed_at(Some(1_700_000_000_000)),
            SetupState::Completed
        );
        assert!(!SetupState::Completed.is_pending());
    }
}
