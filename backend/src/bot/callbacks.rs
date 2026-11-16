use crate::domain::GroupId;

pub const MENU_CREATE: &str = "menu:create";
pub const MENU_GROUPS: &str = "menu:groups";
pub const MENU_HELP: &str = "menu:help";
const GROUP_PREFIX: &str = "group:";

/// Parsed inline-keyboard action. Group IDs from callback data are revalidated
/// against the database before use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackAction {
    MenuCreate,
    MenuGroups,
    MenuHelp,
    SelectGroup(GroupId),
}

pub fn parse(data: &str) -> Option<CallbackAction> {
    match data {
        MENU_CREATE => Some(CallbackAction::MenuCreate),
        MENU_GROUPS => Some(CallbackAction::MenuGroups),
        MENU_HELP => Some(CallbackAction::MenuHelp),
        other => other
            .strip_prefix(GROUP_PREFIX)
            .and_then(|raw| raw.parse::<i64>().ok())
            .and_then(|id| GroupId::new(id).ok())
            .map(CallbackAction::SelectGroup),
    }
}

pub fn select_group(group: GroupId) -> String {
    format!("{GROUP_PREFIX}{}", group.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_menu_actions() {
        assert_eq!(parse(MENU_CREATE), Some(CallbackAction::MenuCreate));
        assert_eq!(parse(MENU_GROUPS), Some(CallbackAction::MenuGroups));
        assert_eq!(parse(MENU_HELP), Some(CallbackAction::MenuHelp));
    }

    #[test]
    fn parses_and_builds_group_selection() {
        let group = GroupId::new(42).unwrap();
        let data = select_group(group);
        assert_eq!(data, "group:42");
        assert_eq!(parse(&data), Some(CallbackAction::SelectGroup(group)));
    }

    #[test]
    fn rejects_unknown_and_invalid_payloads() {
        for data in [
            "",
            "menu:unknown",
            "group:",
            "group:0",
            "group:-3",
            "group:abc",
            "group:999999999999999999999",
        ] {
            assert_eq!(parse(data), None, "{data:?} must be rejected");
        }
    }
}
