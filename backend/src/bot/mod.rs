//! Telegram bot interface. Handlers are thin: all rules live in services.

pub mod callbacks;
pub mod keyboards;
pub mod notifications;
pub mod sessions;
pub mod texts;

use std::{sync::Arc, time::Instant};

use chrono::Utc;
use sqlx::SqlitePool;
use teloxide::{
    RequestError,
    dispatching::UpdateHandler,
    dptree,
    payloads::{EditMessageText, SendMessage},
    prelude::*,
    requests::JsonRequest,
    types::{
        BotCommand, CallbackQuery, CallbackQueryId, ChatId, ForceReply, InlineKeyboardButton,
        InlineKeyboardMarkup, InputFile, LinkPreviewOptions, Message, MessageId,
        User as TelegramUser, WebAppInfo,
    },
    utils::command::BotCommands,
};
use url::Url;

use crate::{
    domain::{GroupId, GroupName, IanaTimezone, InvitationId, InvitationToken, TelegramId, UserId},
    services::{
        best_time, calendar as calendar_service,
        groups::{self, GroupError, GroupSummary},
        invitations::{self, InvitationError, JoinOutcome, RevokeOutcome},
        members::{self, MembersError},
        membership::{self, LeaveError},
        timetable,
        users::{self, TelegramProfile, User},
    },
};

use self::{
    callbacks::CallbackAction,
    sessions::{BeginCreation, BotSessions, CreationState},
};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Доступные команды")]
enum Command {
    #[command(description = "Начать работу")]
    Start,
    #[command(description = "Создать группу")]
    Create,
    #[command(description = "Мои группы")]
    Groups,
    #[command(description = "Открыть календарь")]
    Calendar,
    #[command(description = "Пригласить участников")]
    Invite,
    #[command(description = "Участники группы")]
    Members,
    #[command(description = "Лучшее время")]
    BestTime,
    #[command(description = "PNG-расписание группы")]
    Timetable,
    #[command(description = "Покинуть группу")]
    Leave,
    #[command(description = "Передать владение")]
    Transfer,
    #[command(description = "Удалить группу")]
    Delete,
    #[command(description = "Справка")]
    Help,
}

/// Runtime settings that handlers need besides the database.
#[derive(Debug, Clone)]
pub struct BotSettings {
    pub public_base_url: Option<String>,
}

/// Bot messages frequently contain user-controlled group names, including URLs.
/// Telegram must never turn those names into link previews or media embeds.
fn disabled_link_preview() -> LinkPreviewOptions {
    LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_small_media: false,
        prefer_large_media: false,
        show_above_text: false,
    }
}

trait SafeMessaging {
    fn send_text(&self, chat: ChatId, text: impl Into<String>) -> JsonRequest<SendMessage>;
    fn edit_text(
        &self,
        chat: ChatId,
        message_id: MessageId,
        text: impl Into<String>,
    ) -> JsonRequest<EditMessageText>;
}

impl SafeMessaging for Bot {
    fn send_text(&self, chat: ChatId, text: impl Into<String>) -> JsonRequest<SendMessage> {
        self.send_message(chat, text)
            .link_preview_options(disabled_link_preview())
    }

    fn edit_text(
        &self,
        chat: ChatId,
        message_id: MessageId,
        text: impl Into<String>,
    ) -> JsonRequest<EditMessageText> {
        self.edit_message_text(chat, message_id, text)
            .link_preview_options(disabled_link_preview())
    }
}

/// Runs long polling until the dispatcher stops.
pub async fn run(bot: Bot, pool: SqlitePool, settings: BotSettings) {
    // Command registration is cosmetic; do not delay long polling for it.
    let command_bot = bot.clone();
    tokio::spawn(async move { register_commands(&command_bot).await });

    tracing::info!("telegram bot long polling started");

    let sessions = Arc::new(BotSessions::new());
    let settings = Arc::new(settings);
    let mut dispatcher = Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![pool, sessions, settings])
        .build();

    dispatcher.dispatch().await;
    tracing::warn!("telegram bot dispatcher stopped");
}

async fn register_commands(bot: &Bot) {
    let commands = vec![
        BotCommand::new("start", "Начать работу"),
        BotCommand::new("create", "Создать группу"),
        BotCommand::new("groups", "Мои группы"),
        BotCommand::new("calendar", "Открыть календарь"),
        BotCommand::new("invite", "Пригласить участников"),
        BotCommand::new("members", "Участники группы"),
        BotCommand::new("besttime", "Лучшее время"),
        BotCommand::new("timetable", "PNG-расписание группы"),
        BotCommand::new("leave", "Покинуть группу"),
        BotCommand::new("transfer", "Передать владение"),
        BotCommand::new("delete", "Удалить группу"),
        BotCommand::new("help", "Справка"),
    ];

    if bot.set_my_commands(commands).await.is_err() {
        // Never log the raw request error: HTTP errors may embed the token URL.
        tracing::warn!("failed to register Telegram bot commands");
    }
}

fn schema() -> UpdateHandler<RequestError> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(handle_command),
                )
                .branch(dptree::filter(is_unparsed_command).endpoint(handle_unknown_command))
                .branch(dptree::filter(is_plain_text).endpoint(handle_text)),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback))
}

fn is_unparsed_command(message: Message) -> bool {
    message
        .text()
        .is_some_and(|text| text.trim_start().starts_with('/'))
}

fn is_plain_text(message: Message) -> bool {
    message
        .text()
        .is_some_and(|text| !text.trim_start().starts_with('/'))
}

async fn handle_command(
    bot: Bot,
    message: Message,
    command: Command,
    pool: SqlitePool,
    sessions: Arc<BotSessions>,
    settings: Arc<BotSettings>,
) -> Result<(), RequestError> {
    let chat = message.chat.id;
    let Some(from) = message.from.as_ref() else {
        return Ok(());
    };

    let user = match load_user(&pool, from).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Telegram user");
            bot.send_text(chat, texts::PROFILE_ERROR).await?;
            return Ok(());
        }
    };

    if !matches!(&command, Command::Create) {
        sessions.clear_creation(user.id);
    }

    match command {
        Command::Start => match parse_start_payload(&message) {
            StartPayload::Join(token) => {
                send_invitation_preview(&bot, chat, &pool, user.id, &token).await?;
            }
            StartPayload::InvalidJoin => {
                bot.send_text(chat, texts::INVALID_INVITATION).await?;
            }
            StartPayload::None => {
                bot.send_text(chat, texts::start(&texts::display_name(&user)))
                    .reply_markup(keyboards::main_menu())
                    .await?;
            }
        },
        Command::Help => {
            bot.send_text(chat, texts::help())
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        Command::Create => {
            start_group_creation(&bot, chat, user.id, &sessions).await?;
        }
        Command::Groups => {
            send_groups(&bot, chat, user.id, &pool, &sessions).await?;
        }
        Command::Calendar => {
            send_calendar_link(&bot, chat, &pool, &sessions, &user, &settings).await?;
        }
        Command::Invite => {
            send_invitation(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::Members => {
            send_members(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::BestTime => {
            send_best_time(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::Timetable => {
            send_timetable(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::Leave => {
            request_leave(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::Transfer => {
            request_transfer(&bot, chat, &pool, &sessions, &user).await?;
        }
        Command::Delete => {
            request_delete(&bot, chat, &pool, &sessions, &user).await?;
        }
    }

    Ok(())
}

async fn handle_unknown_command(bot: Bot, message: Message) -> Result<(), RequestError> {
    bot.send_text(message.chat.id, texts::UNKNOWN_COMMAND)
        .await?;
    Ok(())
}

async fn handle_text(
    bot: Bot,
    message: Message,
    pool: SqlitePool,
    sessions: Arc<BotSessions>,
) -> Result<(), RequestError> {
    let chat = message.chat.id;
    let Some(from) = message.from.as_ref() else {
        return Ok(());
    };

    let user = match load_user(&pool, from).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Telegram user");
            bot.send_text(chat, texts::PROFILE_ERROR).await?;
            return Ok(());
        }
    };

    match sessions.creation_state(user.id, Instant::now()) {
        CreationState::NotStarted => {}
        CreationState::Expired => {
            bot.send_text(chat, texts::CREATE_EXPIRED).await?;
        }
        CreationState::Awaiting => {
            let Some(raw_name) = message.text() else {
                return Ok(());
            };

            let name = match GroupName::parse(raw_name) {
                Ok(name) => name,
                Err(error) => {
                    bot.send_text(chat, texts::name_error(&error)).await?;
                    return Ok(());
                }
            };

            sessions.clear_creation(user.id);
            match groups::create_with_owner(&pool, &name, user.id).await {
                Ok(group) => {
                    sessions.set_active_group(user.id, group.id);
                    let invite_url = create_invite_url(&bot, &pool, user.id, group.id).await;
                    bot.send_text(
                        chat,
                        texts::group_created(name.as_str(), invite_url.as_deref()),
                    )
                    .reply_markup(keyboards::main_menu())
                    .await?;
                }
                Err(error) => {
                    tracing::error!(error = %error, "failed to create group");
                    bot.send_text(chat, texts::CREATE_ERROR).await?;
                }
            }
        }
    }

    Ok(())
}

async fn handle_callback(
    bot: Bot,
    query: CallbackQuery,
    pool: SqlitePool,
    sessions: Arc<BotSessions>,
    settings: Arc<BotSettings>,
) -> Result<(), RequestError> {
    let Some(chat) = query.message.as_ref().map(|message| message.chat().id) else {
        answer_alert(&bot, &query.id, texts::CALLBACK_UNAVAILABLE).await?;
        return Ok(());
    };
    let message_id = query.message.as_ref().map(|message| message.id());

    let user = match load_user(&pool, &query.from).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Telegram user");
            answer_alert(&bot, &query.id, texts::PROFILE_ERROR).await?;
            return Ok(());
        }
    };

    let Some(action) = query.data.as_deref().and_then(callbacks::parse) else {
        answer_alert(&bot, &query.id, texts::CALLBACK_UNAVAILABLE).await?;
        return Ok(());
    };

    match action {
        CallbackAction::MenuCreate => {
            answer_silent(&bot, &query.id).await?;
            start_group_creation(&bot, chat, user.id, &sessions).await?;
        }
        CallbackAction::MenuGroups => {
            answer_silent(&bot, &query.id).await?;
            send_groups(&bot, chat, user.id, &pool, &sessions).await?;
        }
        CallbackAction::MenuCalendar => {
            answer_silent(&bot, &query.id).await?;
            send_calendar_link(&bot, chat, &pool, &sessions, &user, &settings).await?;
        }
        CallbackAction::MenuHelp => {
            answer_silent(&bot, &query.id).await?;
            bot.send_text(chat, texts::help())
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        CallbackAction::SelectGroup(group_id) => {
            select_group(&bot, chat, &pool, &sessions, &user, group_id, &query.id).await?;
        }
        CallbackAction::JoinInvitation(token) => {
            join_invitation(&bot, chat, &pool, &sessions, &user, &token, &query.id).await?;
        }
        CallbackAction::RevokeInvitation(invitation) => {
            revoke_invitation(&bot, chat, message_id, &pool, &user, invitation, &query.id).await?;
        }
        CallbackAction::ConfirmLeave(group_id) => {
            confirm_leave(&bot, chat, &pool, &sessions, &user, group_id, &query.id).await?;
        }
        CallbackAction::SelectTransfer { group, target } => {
            select_transfer(&bot, chat, &pool, &user, group, target, &query.id).await?;
        }
        CallbackAction::ConfirmTransfer { group, target } => {
            confirm_transfer(&bot, chat, &pool, &user, group, target, &query.id).await?;
        }
        CallbackAction::ConfirmDelete(group_id) => {
            confirm_delete(&bot, chat, &pool, &sessions, &user, group_id, &query.id).await?;
        }
        CallbackAction::Cancel => {
            answer(&bot, &query.id, texts::CANCELED).await?;
            bot.send_text(chat, texts::CANCELED)
                .reply_markup(keyboards::main_menu())
                .await?;
        }
    }

    Ok(())
}

async fn start_group_creation(
    bot: &Bot,
    chat: ChatId,
    user: UserId,
    sessions: &BotSessions,
) -> Result<(), RequestError> {
    let prompt = match sessions.begin_creation(user, Instant::now()) {
        BeginCreation::Started => texts::CREATE_PROMPT,
        BeginCreation::AlreadyPending => texts::CREATE_PROMPT_AGAIN,
    };

    bot.send_text(chat, prompt)
        .reply_markup(ForceReply::new().input_field_placeholder(Some("Название группы".to_owned())))
        .await?;

    Ok(())
}

async fn send_groups(
    bot: &Bot,
    chat: ChatId,
    user: UserId,
    pool: &SqlitePool,
    sessions: &BotSessions,
) -> Result<(), RequestError> {
    match groups::list_for_user(pool, user).await {
        Ok(groups) if groups.is_empty() => {
            bot.send_text(chat, texts::GROUPS_EMPTY)
                .reply_markup(keyboards::groups_menu(&[]))
                .await?;
        }
        Ok(groups) => {
            let active = sessions.active_group(user);
            let mut body = String::from(texts::GROUPS_HEADER);
            for group in &groups {
                body.push('\n');
                body.push_str(&texts::group_line(group, Some(group.id) == active));
            }

            bot.send_text(chat, body)
                .reply_markup(keyboards::groups_menu(&groups))
                .await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to list groups");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }

    Ok(())
}

async fn send_active_group(
    bot: &Bot,
    chat: ChatId,
    group: &GroupSummary,
) -> Result<(), RequestError> {
    bot.send_text(chat, texts::group_details(group))
        .reply_markup(keyboards::main_menu())
        .await?;
    Ok(())
}

/// Resolves the active group, or asks the user to choose when several exist.
async fn resolve_group(
    bot: &Bot,
    chat: ChatId,
    user: UserId,
    pool: &SqlitePool,
    sessions: &BotSessions,
) -> Result<Option<GroupSummary>, RequestError> {
    if let Some(group_id) = sessions.active_group(user) {
        match groups::get_for_user(pool, user, group_id).await {
            Ok(group) => return Ok(Some(group)),
            Err(GroupError::NotAMember(_)) => sessions.clear_active_group(user),
            Err(error) => {
                tracing::error!(error = %error, "failed to load active group");
                bot.send_text(chat, texts::GENERIC_ERROR).await?;
                return Ok(None);
            }
        }
    }

    match groups::list_for_user(pool, user).await {
        Ok(groups) if groups.is_empty() => {
            bot.send_text(chat, texts::GROUPS_EMPTY)
                .reply_markup(keyboards::groups_menu(&[]))
                .await?;
            Ok(None)
        }
        Ok(mut groups) if groups.len() == 1 => {
            let group = groups.remove(0);
            sessions.set_active_group(user, group.id);
            Ok(Some(group))
        }
        Ok(groups) => {
            bot.send_text(chat, texts::GROUP_PICK_PROMPT)
                .reply_markup(keyboards::groups_menu(&groups))
                .await?;
            Ok(None)
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to list groups");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
            Ok(None)
        }
    }
}

async fn select_group(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
    group_id: GroupId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match groups::get_for_user(pool, user.id, group_id).await {
        Ok(group) => {
            sessions.set_active_group(user.id, group.id);
            answer(bot, query_id, texts::GROUP_SELECTED_TOAST).await?;
            send_active_group(bot, chat, &group).await?;
        }
        Err(GroupError::NotAMember(_)) => {
            answer_alert(bot, query_id, texts::GROUP_NOT_AVAILABLE).await?;
            bot.send_text(chat, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to load selected group");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn send_calendar_link(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
    settings: &BotSettings,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };

    let Some(base_url) = settings.public_base_url.as_deref() else {
        bot.send_text(chat, texts::MINI_APP_UNAVAILABLE).await?;
        return Ok(());
    };

    let mut url = match Url::parse(base_url) {
        Ok(url) => url,
        Err(_) => {
            bot.send_text(chat, texts::MINI_APP_UNAVAILABLE).await?;
            return Ok(());
        }
    };
    url.query_pairs_mut()
        .append_pair("group", &group.id.get().to_string());

    let button = InlineKeyboardButton::web_app("Открыть календарь", WebAppInfo { url });

    bot.send_text(chat, texts::calendar_link(group.name.as_str()))
        .reply_markup(InlineKeyboardMarkup::new(vec![vec![button]]))
        .await?;
    Ok(())
}

/// Creates a reusable invitation and returns its Telegram deep link.
/// Group creation must still succeed if the link cannot be built, so failures
/// are logged and reported as `None` instead of surfacing an error.
async fn create_invite_url(
    bot: &Bot,
    pool: &SqlitePool,
    owner: UserId,
    group: GroupId,
) -> Option<String> {
    let username = match bot.get_me().await {
        Ok(me) => me.username.clone(),
        Err(error) => {
            tracing::warn!(error = %error, "failed to load bot username for invitation");
            return None;
        }
    };
    let Some(username) = username else {
        tracing::warn!("bot username is not set; cannot build invitation link");
        return None;
    };

    match invitations::create(pool, owner, group, None).await {
        Ok(created) => Some(format!(
            "https://t.me/{username}?start={}",
            created.token.start_payload()
        )),
        Err(error) => {
            tracing::error!(error = %error, "failed to create invitation for new group");
            None
        }
    }
}

async fn send_invitation(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    if !group.is_owner() {
        bot.send_text(chat, texts::NOT_OWNER).await?;
        return Ok(());
    }

    let me = match bot.get_me().await {
        Ok(me) => me,
        Err(_) => {
            bot.send_text(chat, texts::INVITATION_LINK_ERROR).await?;
            return Ok(());
        }
    };
    let Some(username) = me.username.as_deref() else {
        bot.send_text(chat, texts::INVITATION_LINK_ERROR).await?;
        return Ok(());
    };

    match invitations::create(pool, user.id, group.id, None).await {
        Ok(created) => {
            let url = format!(
                "https://t.me/{username}?start={}",
                created.token.start_payload()
            );
            bot.send_text(chat, texts::invitation_link(group.name.as_str(), &url))
                .reply_markup(keyboards::invitation_actions(created.id))
                .await?;
        }
        Err(InvitationError::NotOwner(_)) => {
            bot.send_text(chat, texts::NOT_OWNER).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to create invitation");
            bot.send_text(chat, texts::INVITATION_ERROR).await?;
        }
    }

    Ok(())
}

async fn send_invitation_preview(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    viewer: UserId,
    token: &InvitationToken,
) -> Result<(), RequestError> {
    match invitations::preview(pool, token, viewer).await {
        Ok(preview) => {
            let can_join = !preview.is_member && !preview.is_full;
            bot.send_text(chat, texts::invitation_preview(&preview))
                .reply_markup(keyboards::invitation_preview_actions(
                    token,
                    can_join,
                    preview.is_member,
                ))
                .await?;
        }
        Err(
            InvitationError::InvalidToken | InvitationError::Expired | InvitationError::Revoked,
        ) => {
            bot.send_text(chat, texts::INVALID_INVITATION).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to load invitation preview");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn join_invitation(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
    token: &InvitationToken,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match invitations::join(pool, token, user.id).await {
        Ok(JoinOutcome::Joined {
            group_id,
            group_name,
        }) => {
            sessions.set_active_group(user.id, group_id);
            answer(bot, query_id, texts::JOINED_TOAST).await?;
            bot.send_text(chat, texts::joined(&group_name))
                .reply_markup(keyboards::main_menu())
                .await?;
            notifications::joined(bot, pool, group_id, &group_name, &texts::display_name(user))
                .await;
        }
        Ok(JoinOutcome::AlreadyMember {
            group_id,
            group_name,
        }) => {
            sessions.set_active_group(user.id, group_id);
            answer(bot, query_id, texts::ALREADY_MEMBER_TOAST).await?;
            bot.send_text(chat, texts::already_member(&group_name))
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        Err(InvitationError::GroupFull) => {
            answer_alert(bot, query_id, texts::GROUP_FULL_TOAST).await?;
        }
        Err(
            InvitationError::InvalidToken | InvitationError::Expired | InvitationError::Revoked,
        ) => {
            answer_alert(bot, query_id, texts::INVALID_INVITATION).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to join by invitation");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn revoke_invitation(
    bot: &Bot,
    chat: ChatId,
    message_id: Option<MessageId>,
    pool: &SqlitePool,
    user: &User,
    invitation: InvitationId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match invitations::revoke(pool, user.id, invitation).await {
        Ok(RevokeOutcome::Revoked { group_name }) => {
            answer(bot, query_id, texts::REVOKED_TOAST).await?;
            let text = texts::invitation_revoked(&group_name);
            if let Some(message_id) = message_id {
                if bot
                    .edit_text(chat, message_id, text.as_str())
                    .await
                    .is_err()
                {
                    bot.send_text(chat, text).await?;
                }
            } else {
                bot.send_text(chat, text).await?;
            }
        }
        Ok(RevokeOutcome::AlreadyRevoked { .. }) => {
            answer(bot, query_id, texts::ALREADY_REVOKED_TOAST).await?;
        }
        Err(InvitationError::NotOwner(_)) => {
            answer_alert(bot, query_id, texts::NOT_OWNER).await?;
        }
        Err(InvitationError::InvitationNotFound) => {
            answer_alert(bot, query_id, texts::CALLBACK_UNAVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to revoke invitation");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn send_members(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };

    match members::list_for_group(pool, user.id, group.id).await {
        Ok(list) => {
            bot.send_text(chat, texts::members_list(&group, &list, user.id))
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        Err(MembersError::NotAMember(_)) => {
            bot.send_text(chat, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to list members");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn send_best_time(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    let Some(timezone) = user_timezone(user) else {
        bot.send_text(chat, texts::TIMEZONE_REQUIRED).await?;
        return Ok(());
    };

    let calendar =
        match calendar_service::group_calendar(pool, user.id, group.id, Utc::now(), timezone).await
        {
            Ok(calendar) => calendar,
            Err(error) => {
                tracing::error!(error = %error, "failed to build calendar for best time");
                bot.send_text(chat, texts::GENERIC_ERROR).await?;
                return Ok(());
            }
        };

    let ranked = match best_time::rank(&calendar) {
        Ok(ranked) => ranked,
        Err(error) => {
            tracing::error!(error = %error, "failed to rank best time");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
            return Ok(());
        }
    };

    bot.send_text(chat, texts::best_time(&ranked))
        .reply_markup(keyboards::main_menu())
        .await?;
    Ok(())
}

async fn send_timetable(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    let Some(timezone) = user_timezone(user) else {
        bot.send_text(chat, texts::TIMEZONE_REQUIRED).await?;
        return Ok(());
    };

    let calendar =
        match calendar_service::group_calendar(pool, user.id, group.id, Utc::now(), timezone).await
        {
            Ok(calendar) => calendar,
            Err(error) => {
                tracing::error!(error = %error, "failed to build calendar for timetable");
                bot.send_text(chat, texts::GENERIC_ERROR).await?;
                return Ok(());
            }
        };

    let caption = texts::timetable_caption(
        &calendar.group_name,
        calendar.week.start_date,
        calendar.week.end_date,
        calendar.viewer_timezone.name(),
    );
    let png =
        match tokio::task::spawn_blocking(move || timetable::render_png(&calendar, Utc::now()))
            .await
        {
            Ok(Ok(png)) => png,
            Ok(Err(error)) => {
                tracing::error!(error = %error, "failed to render timetable PNG");
                bot.send_text(chat, texts::GENERIC_ERROR).await?;
                return Ok(());
            }
            Err(error) => {
                tracing::error!(error = %error, "timetable task failed");
                bot.send_text(chat, texts::GENERIC_ERROR).await?;
                return Ok(());
            }
        };

    bot.send_document(chat, InputFile::memory(png).file_name("timetable.png"))
        .caption(caption)
        .await?;

    Ok(())
}

fn user_timezone(user: &User) -> Option<IanaTimezone> {
    user.timezone
        .as_deref()
        .and_then(|raw| IanaTimezone::parse(raw).ok())
}

async fn request_leave(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    if group.is_owner() {
        bot.send_text(chat, texts::OWNER_CANNOT_LEAVE).await?;
        return Ok(());
    }

    bot.send_text(chat, texts::leave_confirm(group.name.as_str()))
        .reply_markup(keyboards::leave_confirmation(group.id))
        .await?;
    Ok(())
}

async fn confirm_leave(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
    group: GroupId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match membership::leave(pool, group, user.id).await {
        Ok(left) => {
            if sessions.active_group(user.id) == Some(group) {
                sessions.clear_active_group(user.id);
            }
            answer(bot, query_id, texts::LEFT_TOAST).await?;
            bot.send_text(chat, texts::left(&left.group_name))
                .reply_markup(keyboards::main_menu())
                .await?;
            notifications::left(
                bot,
                pool,
                group,
                &left.group_name,
                &texts::display_name(user),
            )
            .await;
        }
        Err(LeaveError::OwnerCannotLeave(_)) => {
            answer_alert(bot, query_id, texts::OWNER_CANNOT_LEAVE).await?;
        }
        Err(LeaveError::NotAMember(_) | LeaveError::GroupNotFound(_)) => {
            answer_alert(bot, query_id, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to leave group");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn request_transfer(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    if !group.is_owner() {
        bot.send_text(chat, texts::NOT_OWNER).await?;
        return Ok(());
    }

    match members::list_for_group(pool, user.id, group.id).await {
        Ok(list) => {
            let targets: Vec<_> = list
                .into_iter()
                .filter(|member| !member.is_owner())
                .collect();
            if targets.is_empty() {
                bot.send_text(chat, texts::NO_TRANSFER_TARGETS).await?;
                return Ok(());
            }
            bot.send_text(chat, texts::transfer_select(group.name.as_str()))
                .reply_markup(keyboards::transfer_selection(group.id, &targets))
                .await?;
        }
        Err(MembersError::NotAMember(_)) => {
            bot.send_text(chat, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to list transfer targets");
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn select_transfer(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    user: &User,
    group: GroupId,
    target: UserId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    let group_summary = match groups::get_for_user(pool, user.id, group).await {
        Ok(group) => group,
        Err(GroupError::NotAMember(_)) => {
            answer_alert(bot, query_id, texts::GROUP_NOT_AVAILABLE).await?;
            return Ok(());
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to load group for transfer");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
            return Ok(());
        }
    };

    if !group_summary.is_owner() {
        answer_alert(bot, query_id, texts::NOT_OWNER).await?;
        return Ok(());
    }

    match members::list_for_group(pool, user.id, group).await {
        Ok(list) => {
            let Some(target_member) = list
                .iter()
                .find(|member| member.user_id == target && !member.is_owner())
            else {
                answer_alert(bot, query_id, texts::CALLBACK_UNAVAILABLE).await?;
                return Ok(());
            };
            answer_silent(bot, query_id).await?;
            bot.send_text(
                chat,
                texts::transfer_confirm(
                    group_summary.name.as_str(),
                    &texts::member_name(target_member),
                ),
            )
            .reply_markup(keyboards::transfer_confirmation(group, target))
            .await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to load transfer target");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }

    Ok(())
}

async fn confirm_transfer(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    user: &User,
    group: GroupId,
    target: UserId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match groups::transfer_ownership(pool, user.id, group, target).await {
        Ok(transfer) => {
            answer(bot, query_id, texts::TRANSFERRED_TOAST).await?;

            let target_name = members::list_for_group(pool, user.id, group)
                .await
                .ok()
                .and_then(|list| {
                    list.into_iter()
                        .find(|member| member.user_id == target)
                        .map(|member| texts::member_name(&member))
                })
                .unwrap_or_else(|| "участник".to_owned());

            bot.send_text(chat, texts::transferred(&transfer.group_name, &target_name))
                .reply_markup(keyboards::main_menu())
                .await?;
            notifications::new_owner(bot, pool, group, &transfer.group_name).await;
        }
        Err(GroupError::NotOwner(_)) => {
            answer_alert(bot, query_id, texts::NOT_OWNER).await?;
        }
        Err(GroupError::TargetNotMember(_) | GroupError::InvalidTransferTarget) => {
            answer_alert(bot, query_id, texts::CALLBACK_UNAVAILABLE).await?;
        }
        Err(GroupError::GroupNotFound(_)) => {
            answer_alert(bot, query_id, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to transfer ownership");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn request_delete(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
) -> Result<(), RequestError> {
    let Some(group) = resolve_group(bot, chat, user.id, pool, sessions).await? else {
        return Ok(());
    };
    if !group.is_owner() {
        bot.send_text(chat, texts::NOT_OWNER).await?;
        return Ok(());
    }

    bot.send_text(chat, texts::delete_confirm(group.name.as_str()))
        .reply_markup(keyboards::delete_confirmation(group.id))
        .await?;
    Ok(())
}

async fn confirm_delete(
    bot: &Bot,
    chat: ChatId,
    pool: &SqlitePool,
    sessions: &BotSessions,
    user: &User,
    group: GroupId,
    query_id: &CallbackQueryId,
) -> Result<(), RequestError> {
    match groups::delete_group(pool, user.id, group).await {
        Ok(deleted) => {
            sessions.clear_active_group(user.id);
            answer(bot, query_id, texts::DELETED_TOAST).await?;
            bot.send_text(chat, texts::deleted(&deleted.group_name))
                .reply_markup(keyboards::main_menu())
                .await?;

            let mut recipients = deleted.member_telegram_ids;
            recipients.retain(|telegram_id| *telegram_id != user.telegram_id);
            notifications::group_deleted(bot.clone(), recipients, deleted.group_name);
        }
        Err(GroupError::NotOwner(_)) => {
            answer_alert(bot, query_id, texts::NOT_OWNER).await?;
        }
        Err(GroupError::GroupNotFound(_)) => {
            answer_alert(bot, query_id, texts::GROUP_NOT_AVAILABLE).await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to delete group");
            answer_silent(bot, query_id).await?;
            bot.send_text(chat, texts::GENERIC_ERROR).await?;
        }
    }
    Ok(())
}

async fn answer(bot: &Bot, query_id: &CallbackQueryId, text: &str) -> Result<(), RequestError> {
    bot.answer_callback_query(query_id.clone())
        .text(text)
        .await?;
    Ok(())
}

async fn answer_silent(bot: &Bot, query_id: &CallbackQueryId) -> Result<(), RequestError> {
    bot.answer_callback_query(query_id.clone()).await?;
    Ok(())
}

async fn answer_alert(
    bot: &Bot,
    query_id: &CallbackQueryId,
    text: &str,
) -> Result<(), RequestError> {
    bot.answer_callback_query(query_id.clone())
        .text(text)
        .show_alert(true)
        .await?;
    Ok(())
}

enum StartPayload {
    Join(InvitationToken),
    InvalidJoin,
    None,
}

fn parse_start_payload(message: &Message) -> StartPayload {
    let Some(text) = message.text() else {
        return StartPayload::None;
    };
    let Some(payload) = text.split_whitespace().nth(1) else {
        return StartPayload::None;
    };
    let Some(raw) = payload.strip_prefix("join_") else {
        return StartPayload::None;
    };

    match InvitationToken::from_raw(raw) {
        Ok(token) => StartPayload::Join(token),
        Err(_) => StartPayload::InvalidJoin,
    }
}

#[derive(Debug, thiserror::Error)]
enum UserLoadError {
    #[error("Telegram user id does not fit into i64")]
    TelegramIdRange,
    #[error(transparent)]
    Id(#[from] crate::domain::IdError),
    #[error(transparent)]
    User(#[from] users::UserError),
}

async fn load_user(pool: &SqlitePool, telegram_user: &TelegramUser) -> Result<User, UserLoadError> {
    let raw_id = i64::try_from(telegram_user.id.0).map_err(|_| UserLoadError::TelegramIdRange)?;
    let telegram_id = TelegramId::new(raw_id)?;

    let profile = TelegramProfile::new(
        telegram_id,
        telegram_user.username.clone(),
        Some(telegram_user.first_name.clone()),
        telegram_user.last_name.clone(),
    );

    Ok(users::upsert(pool, &profile).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_name_messages_disable_link_previews() {
        let options = disabled_link_preview();
        assert!(options.is_disabled);
        assert!(options.url.is_none());
        assert_eq!(
            serde_json::to_value(options).unwrap(),
            serde_json::json!({ "is_disabled": true })
        );
    }
}
