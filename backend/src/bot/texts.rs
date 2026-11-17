//! Centralized Russian bot copy. Handlers must not hardcode user-facing strings.

use crate::{
    domain::{GroupNameError, MAX_GROUP_MEMBERS, MAX_GROUP_NAME_CHARS, UserId},
    services::{
        best_time::BestTime, groups::GroupSummary, invitations::InvitationPreview,
        members::MemberSummary, users::User,
    },
};

pub const PROFILE_ERROR: &str = "Не удалось сохранить профиль. Попробуйте позже.";
pub const GENERIC_ERROR: &str = "Что-то пошло не так. Попробуйте позже.";
pub const CREATE_ERROR: &str = "Не удалось создать группу. Попробуйте позже.";
pub const INVITATION_ERROR: &str = "Не удалось создать приглашение. Попробуйте позже.";
pub const INVITATION_LINK_ERROR: &str = "Не удалось получить ссылку-приглашение. Попробуйте позже.";
pub const INVALID_INVITATION: &str = "Приглашение недействительно, отозвано или истекло.";
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
pub const NOT_OWNER: &str = "Это действие доступно только владельцу группы.";
pub const OWNER_CANNOT_LEAVE: &str =
    "Вы владелец группы. Сначала передайте владение другому участнику или удалите группу.";
pub const NO_TRANSFER_TARGETS: &str =
    "В группе нет других участников, которым можно передать владение.";
pub const GROUP_PICK_PROMPT: &str =
    "Групп несколько. Выберите активную группу, затем повторите команду.";
pub const CANCELED: &str = "Действие отменено.";
pub const JOINED_TOAST: &str = "Вы вступили в группу";
pub const ALREADY_MEMBER_TOAST: &str = "Вы уже участник";
pub const GROUP_FULL_TOAST: &str = "В группе нет свободных мест";
pub const LEFT_TOAST: &str = "Вы покинули группу";
pub const TRANSFERRED_TOAST: &str = "Владение передано";
pub const DELETED_TOAST: &str = "Группа удалена";
pub const REVOKED_TOAST: &str = "Приглашение отозвано";
pub const ALREADY_REVOKED_TOAST: &str = "Приглашение уже отозвано";
pub const TIMEZONE_REQUIRED: &str =
    "Сначала выберите часовой пояс в Mini App. Без него расписание нельзя пересчитать.";
pub const MINI_APP_UNAVAILABLE: &str = "Открытие Mini App временно недоступно. Попробуйте позже.";

pub fn start(name: &str) -> String {
    format!(
        "Здравствуйте, {name}!\n\n\
         Raid Scheduler помогает небольшой группе договориться о постоянном времени встреч.\n\n\
         В недельном календаре каждый отмечает полчаса, когда он доступен. \
         Отмеченное значит «доступен», а пустая клетка — «недоступен».\n\n\
         Выберите действие:"
    )
}

pub fn help() -> String {
    "Raid Scheduler — недельный планировщик для небольших групп.\n\n\
     Доступные команды:\n\
     /create — создать группу\n\
     /groups — мои группы и выбор активной\n\
     /calendar — открыть календарь в Mini App\n\
     /invite — пригласить участников (владелец)\n\
     /members — участники и статус календарей\n\
     /besttime — лучшее время по доступности\n\
     /timetable — PNG-расписание группы\n\
     /leave — покинуть группу\n\
     /transfer — передать владение (владелец)\n\
     /delete — удалить группу (владелец)\n\
     /help — эта справка\n\n\
     Отмеченное время означает доступность участника, пустая клетка — недоступность."
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

pub fn group_created(name: &str, invite_url: Option<&str>) -> String {
    match invite_url {
        Some(url) => format!(
            "Группа «{name}» создана. Приглашение по ссылке — {url}\n\n\
             Вы указаны как владелец и первый участник."
        ),
        None => format!(
            "Группа «{name}» создана. Приглашение создать не удалось — \
             откройте /invite, чтобы получить ссылку.\n\n\
             Вы указаны как владелец и первый участник."
        ),
    }
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
    format_name(
        user.first_name.as_deref(),
        user.last_name.as_deref(),
        user.username.as_deref(),
    )
}

pub fn member_name(member: &MemberSummary) -> String {
    format_name(
        member.first_name.as_deref(),
        member.last_name.as_deref(),
        member.username.as_deref(),
    )
}

fn format_name(first: Option<&str>, last: Option<&str>, username: Option<&str>) -> String {
    let first = first.unwrap_or("").trim();
    let last = last.unwrap_or("").trim();

    if !first.is_empty() && !last.is_empty() {
        format!("{first} {last}")
    } else if !first.is_empty() {
        first.to_owned()
    } else if let Some(username) = username {
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

pub fn members_list(group: &GroupSummary, members: &[MemberSummary], viewer: UserId) -> String {
    let mut text = format!(
        "Участники группы «{}» ({} из {}):",
        group.name.as_str(),
        members.len(),
        MAX_GROUP_MEMBERS
    );

    for (index, member) in members.iter().enumerate() {
        text.push_str(&format!(
            "\n\n{}. {} — {}",
            index + 1,
            member_name(member),
            if member.is_owner() {
                "владелец"
            } else {
                "участник"
            }
        ));
        text.push_str(if member.setup_state().is_pending() {
            ", календарь не настроен"
        } else {
            ", календарь настроен"
        });
        if member.user_id == viewer {
            text.push_str(", вы");
        }
    }

    text
}

pub fn invitation_preview(preview: &InvitationPreview) -> String {
    let owner = preview
        .members
        .iter()
        .find(|member| member.is_owner())
        .map(member_name)
        .unwrap_or_else(|| "Участник".to_owned());

    let mut text = format!(
        "Приглашение в группу «{}»\n\n\
         Владелец: {owner}\n\
         Участников: {} из {}\n\n\
         Участники:",
        preview.group_name, preview.member_count, MAX_GROUP_MEMBERS
    );

    for member in &preview.members {
        text.push_str("\n• ");
        text.push_str(&member_name(member));
        if member.is_owner() {
            text.push_str(" (владелец)");
        }
    }

    text.push_str("\n\n");
    if preview.is_member {
        text.push_str("Вы уже участник этой группы.");
    } else if preview.is_full {
        text.push_str("Свободных мест нет.");
    } else {
        text.push_str(
            "Свободные места есть. Просмотр не добавляет вас в группу. Чтобы вступить, нажмите кнопку ниже.",
        );
    }

    text
}

pub fn invitation_link(group_name: &str, url: &str) -> String {
    format!(
        "Ссылка-приглашение в группу «{group_name}»:\n\n\
         {url}\n\n\
         Откройте ссылку и подтвердите вступление. Просмотр не добавляет в группу.\n\
         Ссылка действует, пока вы её не отзовёте."
    )
}

pub fn invitation_revoked(group_name: &str) -> String {
    format!("Приглашение в группу «{group_name}» отозвано.")
}

pub fn joined(group_name: &str) -> String {
    format!("Вы вступили в группу «{group_name}».")
}

pub fn already_member(group_name: &str) -> String {
    format!("Вы уже участник группы «{group_name}».")
}

pub fn leave_confirm(group_name: &str) -> String {
    format!(
        "Покинуть группу «{group_name}»?\n\n\
         Ваши отметки доступности в этой группе будут удалены. Данные в других группах не изменятся."
    )
}

pub fn left(group_name: &str) -> String {
    format!("Вы покинули группу «{group_name}».")
}

pub fn transfer_select(group_name: &str) -> String {
    format!("Выберите участника, которому передать владение группой «{group_name}».")
}

pub fn transfer_confirm(group_name: &str, target_name: &str) -> String {
    format!(
        "Передать владение группой «{group_name}» участнику {target_name}?\n\n\
         После передачи вы останетесь участником, а владельцем станет {target_name}."
    )
}

pub fn transferred(group_name: &str, target_name: &str) -> String {
    format!("Владение группой «{group_name}» передано участнику {target_name}.")
}

pub fn delete_confirm(group_name: &str) -> String {
    format!(
        "Удалить группу «{group_name}»?\n\n\
         Все приглашения, отметки доступности и данные группы будут удалены без возможности восстановления."
    )
}

pub fn deleted(group_name: &str) -> String {
    format!("Группа «{group_name}» удалена.")
}

pub fn notify_joined(member_name: &str, group_name: &str) -> String {
    format!("В группу «{group_name}» вступил новый участник: {member_name}.")
}

pub fn notify_left(member_name: &str, group_name: &str) -> String {
    format!("Участник {member_name} покинул группу «{group_name}».")
}

pub fn notify_preferences_changed(member_name: &str, group_name: &str) -> String {
    format!("Участник {member_name} обновил доступность в группе «{group_name}».")
}

pub fn notify_new_owner(group_name: &str) -> String {
    format!("Вы стали владельцем группы «{group_name}».")
}

pub fn notify_group_deleted(group_name: &str) -> String {
    format!("Группа «{group_name}» удалена владельцем.")
}

pub fn best_time(best: &BestTime) -> String {
    let mut text = format!(
        "Лучшее время в группе «{}»\n\
         Неделя: {} — {}\n\
         Часовой пояс: {}\n\n\
         Настроено участников: {} из {}.",
        best.group_name,
        best.week.start_date.format("%d.%m.%Y"),
        best.week.end_date.format("%d.%m.%Y"),
        best.viewer_timezone.name(),
        best.configured_member_count,
        best.total_member_count
    );

    if best.pending_member_count > 0 {
        text.push_str(&format!(
            "\nНе настроены: {}. Они исключены из рейтинга.",
            participants(best.pending_member_count as i64)
        ));
    }

    if best.configured_member_count == 0 {
        text.push_str("\n\nПока никто не настроил календарь.");
        return text;
    }
    if best.intervals.is_empty() {
        text.push_str("\n\nНет отмеченного времени.");
        return text;
    }

    text.push_str("\n\n");
    for (index, interval) in best.intervals.iter().take(10).enumerate() {
        text.push_str(&format!(
            "{}. {}, {}–{} — {}\n",
            index + 1,
            day_name(interval.day),
            format_time(interval.start_minute),
            format_time(interval.end_minute),
            available_by(interval.count())
        ));
    }
    if best.intervals.len() > 10 {
        text.push_str("\nПоказаны первые 10 интервалов.");
    }

    text.push_str(
        "\n\nЭто отмеченная участниками доступность. \
         Пустая клетка означает недоступность.",
    );
    text
}

pub fn calendar_link(group_name: &str) -> String {
    format!("Откройте Mini App, чтобы настроить или посмотреть календарь группы «{group_name}».")
}

pub fn timetable_caption(
    group_name: &str,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    timezone: &str,
) -> String {
    format!(
        "Расписание группы «{group_name}»\n\
         Неделя: {} — {}\n\
         Часовой пояс: {timezone}",
        start_date.format("%d.%m.%Y"),
        end_date.format("%d.%m.%Y")
    )
}

pub fn day_name(day: u8) -> &'static str {
    match day {
        0 => "Понедельник",
        1 => "Вторник",
        2 => "Среда",
        3 => "Четверг",
        4 => "Пятница",
        5 => "Суббота",
        _ => "Воскресенье",
    }
}

pub fn format_time(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

pub fn available_by(count: usize) -> String {
    if count == 1 {
        "доступен 1 участник".to_owned()
    } else {
        format!("доступны: {}", participants(count as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{GroupId, MemberRole, TelegramId, UserId},
        services::{groups::GroupSummary, invitations::InvitationPreview, members::MemberSummary},
    };

    fn member(
        id: i64,
        first: &str,
        role: MemberRole,
        setup_completed_at: Option<i64>,
    ) -> MemberSummary {
        MemberSummary {
            user_id: UserId::new(id).unwrap(),
            first_name: Some(first.to_owned()),
            last_name: None,
            username: None,
            role,
            color: "#4DD0E1".to_owned(),
            timezone: Some("Asia/Tbilisi".to_owned()),
            setup_completed_at,
            joined_at: 0,
        }
    }

    fn group() -> GroupSummary {
        GroupSummary {
            id: GroupId::new(1).unwrap(),
            name: crate::domain::GroupName::parse("Рейд").unwrap(),
            role: MemberRole::Owner,
            color: "#4DD0E1".to_owned(),
            member_count: 2,
            setup_completed_at: None,
        }
    }

    #[test]
    fn members_list_marks_owner_setup_status_and_viewer() {
        let text = members_list(
            &group(),
            &[
                member(1, "Иван", MemberRole::Owner, None),
                member(2, "Пётр", MemberRole::Member, Some(1)),
            ],
            UserId::new(2).unwrap(),
        );

        assert!(text.contains("1. Иван — владелец, календарь не настроен"));
        assert!(text.contains("2. Пётр — участник, календарь настроен, вы"));
    }

    #[test]
    fn invitation_preview_reports_capacity_and_owner() {
        let preview = InvitationPreview {
            group_id: GroupId::new(1).unwrap(),
            group_name: "Рейд".to_owned(),
            members: vec![
                member(1, "Иван", MemberRole::Owner, None),
                member(2, "Пётр", MemberRole::Member, None),
            ],
            member_count: 2,
            is_full: false,
            is_member: false,
        };

        let text = invitation_preview(&preview);
        assert!(text.contains("Владелец: Иван"));
        assert!(text.contains("Участников: 2 из 20"));
        assert!(text.contains("Свободные места есть"));
        assert!(text.contains("• Иван (владелец)"));

        let full = InvitationPreview {
            is_full: true,
            member_count: MAX_GROUP_MEMBERS as i64,
            ..preview
        };
        assert!(invitation_preview(&full).contains("Свободных мест нет"));
    }

    #[test]
    fn group_created_includes_invitation_link_when_available() {
        let text = group_created("Тестирование", Some("https://t.me/test_bot?start=abc123"));
        assert_eq!(
            text,
            "Группа «Тестирование» создана. Приглашение по ссылке — \
             https://t.me/test_bot?start=abc123\n\n\
             Вы указаны как владелец и первый участник."
        );

        let fallback = group_created("Тестирование", None);
        assert!(fallback.contains("Приглашение создать не удалось"));
        assert!(fallback.contains("/invite"));
    }

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
    fn formats_best_time_primitives() {
        assert_eq!(format_time(0), "00:00");
        assert_eq!(format_time(570), "09:30");
        assert_eq!(format_time(1440), "24:00");
        assert_eq!(day_name(0), "Понедельник");
        assert_eq!(day_name(6), "Воскресенье");
        assert_eq!(available_by(1), "доступен 1 участник");
        assert_eq!(available_by(2), "доступны: 2 участника");
        assert_eq!(available_by(5), "доступны: 5 участников");
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
