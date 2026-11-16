use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::services::groups::GroupSummary;

use super::{callbacks, texts};

pub fn main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "Создать группу",
            callbacks::MENU_CREATE,
        )],
        vec![InlineKeyboardButton::callback(
            "Мои группы",
            callbacks::MENU_GROUPS,
        )],
        vec![InlineKeyboardButton::callback(
            "Помощь",
            callbacks::MENU_HELP,
        )],
    ])
}

pub fn groups_menu(groups: &[GroupSummary]) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = groups
        .iter()
        .map(|group| {
            vec![InlineKeyboardButton::callback(
                group_label(group),
                callbacks::select_group(group.id),
            )]
        })
        .collect();

    rows.push(vec![InlineKeyboardButton::callback(
        "Создать группу",
        callbacks::MENU_CREATE,
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        "Помощь",
        callbacks::MENU_HELP,
    )]);

    InlineKeyboardMarkup::new(rows)
}

fn group_label(group: &GroupSummary) -> String {
    format!(
        "{} · {}",
        texts::truncate(group.name.as_str(), 42),
        group.member_count
    )
}

#[cfg(test)]
mod tests {
    use teloxide::types::InlineKeyboardButtonKind;

    use super::*;
    use crate::domain::{GroupName, MemberRole};

    fn callback_data(markup: &InlineKeyboardMarkup) -> Vec<String> {
        markup
            .inline_keyboard
            .iter()
            .flatten()
            .map(|button| match &button.kind {
                InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
                other => panic!("unexpected button kind: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn main_menu_contains_three_actions() {
        let data = callback_data(&main_menu());
        assert_eq!(
            data,
            vec![
                callbacks::MENU_CREATE.to_owned(),
                callbacks::MENU_GROUPS.to_owned(),
                callbacks::MENU_HELP.to_owned()
            ]
        );
    }

    #[test]
    fn groups_menu_lists_each_group_then_actions() {
        let groups = vec![GroupSummary {
            id: crate::domain::GroupId::new(7).unwrap(),
            name: GroupName::parse("Рейд").unwrap(),
            role: MemberRole::Owner,
            member_count: 3,
            setup_completed_at: None,
        }];

        let markup = groups_menu(&groups);
        let data = callback_data(&markup);

        assert_eq!(data.len(), 3);
        assert_eq!(data[0], "group:7");
        assert_eq!(data[1], callbacks::MENU_CREATE);
        assert_eq!(data[2], callbacks::MENU_HELP);
        assert_eq!(markup.inline_keyboard[0][0].text, "Рейд · 3");
    }
}
