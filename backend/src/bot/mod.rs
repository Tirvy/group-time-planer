//! Telegram bot interface. Handlers are thin: all rules live in services.

pub mod callbacks;
pub mod keyboards;
pub mod sessions;
pub mod texts;

use std::{sync::Arc, time::Instant};

use sqlx::SqlitePool;
use teloxide::{
    RequestError,
    dispatching::UpdateHandler,
    dptree,
    prelude::*,
    types::{BotCommand, CallbackQuery, ChatId, ForceReply, Message, User as TelegramUser},
    utils::command::BotCommands,
};

use crate::{
    domain::{GroupName, TelegramId, UserId},
    services::{
        groups::{self, GroupError, GroupSummary},
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
    #[command(description = "Справка")]
    Help,
}

/// Runs long polling until the dispatcher stops.
pub async fn run(bot: Bot, pool: SqlitePool) {
    // Command registration is cosmetic; do not delay long polling for it.
    let command_bot = bot.clone();
    tokio::spawn(async move { register_commands(&command_bot).await });

    tracing::info!("telegram bot long polling started");

    let sessions = Arc::new(BotSessions::new());
    let mut dispatcher = Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![pool, sessions])
        .build();

    dispatcher.dispatch().await;
    tracing::warn!("telegram bot dispatcher stopped");
}

async fn register_commands(bot: &Bot) {
    let commands = vec![
        BotCommand::new("start", "Начать работу"),
        BotCommand::new("create", "Создать группу"),
        BotCommand::new("groups", "Мои группы"),
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
) -> Result<(), RequestError> {
    let chat = message.chat.id;
    let Some(from) = message.from.as_ref() else {
        return Ok(());
    };

    let user = match load_user(&pool, from).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Telegram user");
            bot.send_message(chat, texts::PROFILE_ERROR).await?;
            return Ok(());
        }
    };

    match command {
        Command::Start => {
            sessions.clear_creation(user.id);
            bot.send_message(chat, texts::start(&texts::display_name(&user)))
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        Command::Help => {
            sessions.clear_creation(user.id);
            bot.send_message(chat, texts::help())
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        Command::Create => {
            start_group_creation(&bot, chat, user.id, &sessions).await?;
        }
        Command::Groups => {
            sessions.clear_creation(user.id);
            send_groups(&bot, chat, user.id, &pool, &sessions).await?;
        }
    }

    Ok(())
}

async fn handle_unknown_command(bot: Bot, message: Message) -> Result<(), RequestError> {
    bot.send_message(message.chat.id, texts::UNKNOWN_COMMAND)
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
            bot.send_message(chat, texts::PROFILE_ERROR).await?;
            return Ok(());
        }
    };

    match sessions.creation_state(user.id, Instant::now()) {
        CreationState::NotStarted => {}
        CreationState::Expired => {
            bot.send_message(chat, texts::CREATE_EXPIRED).await?;
        }
        CreationState::Awaiting => {
            let Some(raw_name) = message.text() else {
                return Ok(());
            };

            let name = match GroupName::parse(raw_name) {
                Ok(name) => name,
                Err(error) => {
                    bot.send_message(chat, texts::name_error(&error)).await?;
                    return Ok(());
                }
            };

            sessions.clear_creation(user.id);
            match groups::create_with_owner(&pool, &name, user.id).await {
                Ok(group) => {
                    sessions.set_active_group(user.id, group.id);
                    bot.send_message(chat, texts::group_created(name.as_str()))
                        .reply_markup(keyboards::main_menu())
                        .await?;
                }
                Err(error) => {
                    tracing::error!(error = %error, "failed to create group");
                    bot.send_message(chat, texts::CREATE_ERROR).await?;
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
) -> Result<(), RequestError> {
    let Some(chat) = query.message.as_ref().map(|message| message.chat().id) else {
        bot.answer_callback_query(query.id)
            .text(texts::CALLBACK_UNAVAILABLE)
            .await?;
        return Ok(());
    };

    let user = match load_user(&pool, &query.from).await {
        Ok(user) => user,
        Err(error) => {
            tracing::error!(error = %error, "failed to upsert Telegram user");
            bot.answer_callback_query(query.id)
                .text(texts::PROFILE_ERROR)
                .await?;
            return Ok(());
        }
    };

    let Some(action) = query.data.as_deref().and_then(callbacks::parse) else {
        bot.answer_callback_query(query.id)
            .text(texts::CALLBACK_UNAVAILABLE)
            .await?;
        return Ok(());
    };

    match action {
        CallbackAction::MenuCreate => {
            bot.answer_callback_query(query.id).await?;
            start_group_creation(&bot, chat, user.id, &sessions).await?;
        }
        CallbackAction::MenuGroups => {
            bot.answer_callback_query(query.id).await?;
            send_groups(&bot, chat, user.id, &pool, &sessions).await?;
        }
        CallbackAction::MenuHelp => {
            bot.answer_callback_query(query.id).await?;
            bot.send_message(chat, texts::help())
                .reply_markup(keyboards::main_menu())
                .await?;
        }
        CallbackAction::SelectGroup(group_id) => {
            match groups::get_for_user(&pool, user.id, group_id).await {
                Ok(group) => {
                    sessions.set_active_group(user.id, group.id);
                    bot.answer_callback_query(query.id)
                        .text(texts::GROUP_SELECTED_TOAST)
                        .await?;
                    send_active_group(&bot, chat, &group).await?;
                }
                Err(GroupError::NotAMember(_)) => {
                    bot.answer_callback_query(query.id)
                        .text(texts::GROUP_NOT_AVAILABLE)
                        .await?;
                    bot.send_message(chat, texts::GROUP_NOT_AVAILABLE).await?;
                }
                Err(error) => {
                    tracing::error!(error = %error, "failed to load selected group");
                    bot.answer_callback_query(query.id).await?;
                    bot.send_message(chat, texts::GENERIC_ERROR).await?;
                }
            }
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

    bot.send_message(chat, prompt)
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
            bot.send_message(chat, texts::GROUPS_EMPTY)
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

            bot.send_message(chat, body)
                .reply_markup(keyboards::groups_menu(&groups))
                .await?;
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to list groups");
            bot.send_message(chat, texts::GENERIC_ERROR).await?;
        }
    }

    Ok(())
}

async fn send_active_group(
    bot: &Bot,
    chat: ChatId,
    group: &GroupSummary,
) -> Result<(), RequestError> {
    bot.send_message(chat, texts::group_details(group))
        .reply_markup(keyboards::main_menu())
        .await?;
    Ok(())
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
