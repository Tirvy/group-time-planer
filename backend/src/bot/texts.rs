//! Centralized Russian bot copy. Handlers must not hardcode user-facing strings.

use crate::{
    domain::{GroupNameError, MAX_GROUP_MEMBERS, MAX_GROUP_NAME_CHARS},
    services::{groups::GroupSummary, users::User},
};

pub const PROFILE_ERROR: &str = "Не удалось сохранить профиль. Попробуйте позже.";
pub const GENERIC_ERROR: &str = "Что-то пошло не так. Попробуйте позже.";
pub const CREATE_ERROR: &str = "Не удалось создать группу. Попробуйте позже.";
pub const UNKNOWN_COMMAND: &str =
    "Неизвестная команда. Откройте /help, чтобы увидеть доступные команды.";
pub const CREATE_PROMPT: &str = "Отправьте название новой группы (до 64 символов).";
pub const CREATE_PROMPT_AGAIN: &str =
    "Вы уже начали создание группы. Отправьте название (до 64 символов) или другую команду.";
pub const CREATE_EXPIRED: &str =
    "Время ожидания названия истекло. Отправьте /create, чтобы начать заново.";
pub const GROUPS_EMPTY: &str =
    "У вас пока нет групп. Создайте первую группу — в ней может быть до 20 участников.";
pub const GROUPS_HEADER: &str = "Ваши группы:";
pub const GROUP_NOT_AVAILABLE: &str =
    "Группа недоступна. Возможно, вы больше не являетесь её участником.";
pub const CALLBACK_UNAVAILABLE: &str = "Кнопка устарела. Откройте /groups или /help.";
pub const GROUP_SELECTED_TOAST: &str = "Группа выбрана";
pub const NO_GROUPS_TOAST: &str = "У вас пока нет групп";

pub fn start(name: &str) -> String {
    format!(
        "Здравствуйте, {name}!\n\n\
         Raid Scheduler помогает небольшой группе договориться о постоянном времени встреч.\n\n\
         В недельном календаре каждый отмечает удобные полчаса. Отмеченное значит \
         «предпочитаю», а пустая клетка — «время не указано», а не «недоступно».\n\n\
         Выберите действие:"
    )
}

pub fn help() -> String {
    "Raid Scheduler — недельный планировщик для небольших групп.\n\n\
     Доступные команды:\n\
     /create — создать группу\n\
     /groups — мои группы и выбор активной\n\
     /help — эта справка\n\n\
     Отмеченное время означает предпочтение, а не подтверждённую доступность."
        .to_owned()
}

pub fn name_error(error: &GroupNameError) -> String {
    match error {
        GroupNameError::Empty => {
            "Название не может быть пустым. Отправьте название ещё раз.".to_owned()
        }
        GroupNameError::TooLong { .. } => format!(
            "Название слишком длинное: максимум {MAX_GROUP_NAME_CHARS} символа. \
             Отправьте название короче."
        ),
        GroupNameError::ControlCharacter => {
            "Название содержит недопустимые символы. Попробуйте другое название.".to_owned()
        }
    }
}

pub fn group_created(name: &str) -> String {
    format!(
        "Группа «{name}» создана. Вы указаны как владелец и первый участник.\n\n\
         Откройте /groups, чтобы увидеть её в списке."
    )
}

pub fn group_line(group: &GroupSummary, active: bool) -> String {
    let active_mark = if active { ", активная" } else { "" };
    format!(
        "• {} — {}, {}{}{}",
        group.name.as_str(),
        participants(group.member_count),
        role_label(group),
        setup_suffix(group),
        active_mark
    )
}

pub fn group_details(group: &GroupSummary) -> String {
    format!(
        "Активная группа: {}\n\n\
         Участников: {} из {}\n\
         Ваша роль: {}\n\
         Ваш календарь: {}",
        group.name.as_str(),
        group.member_count,
        MAX_GROUP_MEMBERS,
        role_label(group),
        if group.setup_state().is_pending() {
            "не настроен"
        } else {
            "настроен"
        }
    )
}

fn role_label(group: &GroupSummary) -> &'static str {
    if group.is_owner() {
        "владелец"
    } else {
        "участник"
    }
}

fn setup_suffix(group: &GroupSummary) -> &'static str {
    if group.setup_state().is_pending() {
        ", календарь не настроен"
    } else {
        ", календарь настроен"
    }
}

/// Russian pluralization for member counts.
pub fn participants(count: i64) -> String {
    let word = if (11..=14).contains(&(count % 100)) {
        "участников"
    } else {
        match count % 10 {
            1 => "участник",
            2..=4 => "участника",
            _ => "участников",
        }
    };
    format!("{count} {word}")
}

/// Prefers a real display name, then username, then a neutral fallback.
pub fn display_name(user: &User) -> String {
    let first = user.first_name.as_deref().unwrap_or("").trim();
    let last = user.last_name.as_deref().unwrap_or("").trim();

    if !first.is_empty() && !last.is_empty() {
        format!("{first} {last}")
    } else if !first.is_empty() {
        first.to_owned()
    } else if let Some(username) = user.username.as_deref() {
        let username = username.trim();
        if username.is_empty() {
            "Участник".to_owned()
        } else {
            format!("@{username}")
        }
    } else {
        "Участник".to_owned()
    }
}

/// Truncates by characters, not bytes, so Cyrillic labels stay intact.
pub fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TelegramId, UserId};

    #[test]
    fn pluralizes_participants_in_russian() {
        assert_eq!(participants(1), "1 участник");
        assert_eq!(participants(2), "2 участника");
        assert_eq!(participants(4), "4 участника");
        assert_eq!(participants(5), "5 участников");
        assert_eq!(participants(11), "11 участников");
        assert_eq!(participants(14), "14 участников");
        assert_eq!(participants(21), "21 участник");
        assert_eq!(participants(22), "22 участника");
        assert_eq!(participants(25), "25 участников");
        assert_eq!(participants(111), "111 участников");
    }

    #[test]
    fn truncates_by_characters_and_adds_ellipsis() {
        assert_eq!(truncate("коротко", 20), "коротко");
        assert_eq!(truncate("длинное название", 4), "длин…");
    }

    #[test]
    fn display_name_prefers_full_name_then_username() {
        let base = |first: Option<&str>, last: Option<&str>, username: Option<&str>| User {
            id: UserId::new(1).unwrap(),
            telegram_id: TelegramId::new(10).unwrap(),
            username: username.map(str::to_owned),
            first_name: first.map(str::to_owned),
            last_name: last.map(str::to_owned),
            timezone: None,
        };

        assert_eq!(
            display_name(&base(Some("Иван"), Some("Петров"), Some("ivan"))),
            "Иван Петров"
        );
        assert_eq!(
            display_name(&base(Some("Иван"), None, Some("ivan"))),
            "Иван"
        );
        assert_eq!(display_name(&base(None, None, Some("ivan"))), "@ivan");
        assert_eq!(display_name(&base(None, None, None)), "Участник");
    }
}
