use crate::domain::{GroupId, InvitationId, InvitationToken, UserId};

pub const MENU_CREATE: &str = "menu:create";
pub const MENU_GROUPS: &str = "menu:groups";
pub const MENU_CALENDAR: &str = "menu:calendar";
pub const MENU_HELP: &str = "menu:help";
pub const CANCEL: &str = "cancel";

/// Parsed inline-keyboard action. Identifiers from callback data are always
/// revalidated against the database before use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackAction {
    MenuCreate,
    MenuGroups,
    MenuCalendar,
    MenuHelp,
    SelectGroup(GroupId),
    JoinInvitation(InvitationToken),
    RevokeInvitation(InvitationId),
    ConfirmLeave(GroupId),
    SelectTransfer { group: GroupId, target: UserId },
    ConfirmTransfer { group: GroupId, target: UserId },
    ConfirmDelete(GroupId),
    Cancel,
}

pub fn parse(data: &str) -> Option<CallbackAction> {
    match data {
        MENU_CREATE => Some(CallbackAction::MenuCreate),
        MENU_GROUPS => Some(CallbackAction::MenuGroups),
        MENU_CALENDAR => Some(CallbackAction::MenuCalendar),
        MENU_HELP => Some(CallbackAction::MenuHelp),
        CANCEL => Some(CallbackAction::Cancel),
        other => {
            if let Some(raw) = other.strip_prefix("group:") {
                return parse_group(raw).map(CallbackAction::SelectGroup);
            }
            if let Some(raw) = other.strip_prefix("join:") {
                return InvitationToken::from_raw(raw)
                    .ok()
                    .map(CallbackAction::JoinInvitation);
            }
            if let Some(raw) = other.strip_prefix("revoke:") {
                return parse_invitation(raw).map(CallbackAction::RevokeInvitation);
            }
            if let Some(raw) = other.strip_prefix("leave:confirm:") {
                return parse_group(raw).map(CallbackAction::ConfirmLeave);
            }
            if let Some(raw) = other.strip_prefix("transfer_confirm:") {
                return parse_pair(raw)
                    .map(|(group, target)| CallbackAction::ConfirmTransfer { group, target });
            }
            if let Some(raw) = other.strip_prefix("transfer:") {
                return parse_pair(raw)
                    .map(|(group, target)| CallbackAction::SelectTransfer { group, target });
            }
            if let Some(raw) = other.strip_prefix("delete:confirm:") {
                return parse_group(raw).map(CallbackAction::ConfirmDelete);
            }
            None
        }
    }
}

pub fn select_group(group: GroupId) -> String {
    format!("group:{}", group.get())
}

pub fn join_invitation(token: &InvitationToken) -> String {
    format!("join:{}", token.expose())
}

pub fn revoke_invitation(invitation: InvitationId) -> String {
    format!("revoke:{}", invitation.get())
}

pub fn confirm_leave(group: GroupId) -> String {
    format!("leave:confirm:{}", group.get())
}

pub fn select_transfer(group: GroupId, target: UserId) -> String {
    format!("transfer:{}:{}", group.get(), target.get())
}

pub fn confirm_transfer(group: GroupId, target: UserId) -> String {
    format!("transfer_confirm:{}:{}", group.get(), target.get())
}

pub fn confirm_delete(group: GroupId) -> String {
    format!("delete:confirm:{}", group.get())
}

fn parse_i64(raw: &str) -> Option<i64> {
    raw.parse::<i64>().ok()
}

fn parse_group(raw: &str) -> Option<GroupId> {
    parse_i64(raw).and_then(|id| GroupId::new(id).ok())
}

fn parse_invitation(raw: &str) -> Option<InvitationId> {
    parse_i64(raw).and_then(|id| InvitationId::new(id).ok())
}

fn parse_pair(raw: &str) -> Option<(GroupId, UserId)> {
    let mut parts = raw.split(':');
    let group = parse_group(parts.next()?)?;
    let target = parse_i64(parts.next()?).and_then(|id| UserId::new(id).ok())?;
    if parts.next().is_some() {
        return None;
    }
    Some((group, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: i64) -> GroupId {
        GroupId::new(id).unwrap()
    }

    fn user(id: i64) -> UserId {
        UserId::new(id).unwrap()
    }

    #[test]
    fn parses_menu_and_cancel_actions() {
        assert_eq!(parse(MENU_CREATE), Some(CallbackAction::MenuCreate));
        assert_eq!(parse(MENU_GROUPS), Some(CallbackAction::MenuGroups));
        assert_eq!(parse(MENU_CALENDAR), Some(CallbackAction::MenuCalendar));
        assert_eq!(parse(MENU_HELP), Some(CallbackAction::MenuHelp));
        assert_eq!(parse(CANCEL), Some(CallbackAction::Cancel));
    }

    #[test]
    fn parses_and_builds_group_and_invitation_actions() {
        let group = group(42);
        assert_eq!(
            parse(&select_group(group)),
            Some(CallbackAction::SelectGroup(group))
        );

        let token = InvitationToken::generate();
        let join = join_invitation(&token);
        assert!(
            join.len() <= 64,
            "Telegram callback data must fit in 64 bytes"
        );
        assert_eq!(parse(&join), Some(CallbackAction::JoinInvitation(token)));

        let invitation = InvitationId::new(9).unwrap();
        assert_eq!(
            parse(&revoke_invitation(invitation)),
            Some(CallbackAction::RevokeInvitation(invitation))
        );
    }

    #[test]
    fn parses_and_builds_lifecycle_actions() {
        let group = group(7);
        let target = user(8);

        assert_eq!(
            parse(&confirm_leave(group)),
            Some(CallbackAction::ConfirmLeave(group))
        );
        assert_eq!(
            parse(&select_transfer(group, target)),
            Some(CallbackAction::SelectTransfer { group, target })
        );
        assert_eq!(
            parse(&confirm_transfer(group, target)),
            Some(CallbackAction::ConfirmTransfer { group, target })
        );
        assert_eq!(
            parse(&confirm_delete(group)),
            Some(CallbackAction::ConfirmDelete(group))
        );
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
            "join:short",
            "revoke:0",
            "leave:confirm:0",
            "transfer:1",
            "transfer:1:0",
            "transfer:1:2:3",
            "delete:confirm:",
        ] {
            assert_eq!(parse(data), None, "{data:?} must be rejected");
        }
    }
}
