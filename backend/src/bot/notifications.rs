//! Best-effort Telegram notifications.
//!
//! Every function is called only after a domain transaction has committed.
//! Delivery failures are logged safely and never propagate or roll back writes.

use std::time::Duration;

use sqlx::SqlitePool;
use teloxide::{prelude::*, types::ChatId};

use crate::{
    domain::{GroupId, TelegramId},
    services::groups,
};

use super::texts;

const DELIVERY_TIMEOUT: Duration = Duration::from_secs(3);

pub async fn joined(
    bot: &Bot,
    pool: &SqlitePool,
    group_id: GroupId,
    group_name: &str,
    member_name: &str,
) {
    notify_owner(
        bot,
        pool,
        group_id,
        texts::notify_joined(member_name, group_name),
    )
    .await;
}

pub async fn left(
    bot: &Bot,
    pool: &SqlitePool,
    group_id: GroupId,
    group_name: &str,
    member_name: &str,
) {
    notify_owner(
        bot,
        pool,
        group_id,
        texts::notify_left(member_name, group_name),
    )
    .await;
}

pub async fn new_owner(bot: &Bot, pool: &SqlitePool, group_id: GroupId, group_name: &str) {
    notify_owner(bot, pool, group_id, texts::notify_new_owner(group_name)).await;
}

pub async fn preferences_changed(
    bot: &Bot,
    pool: &SqlitePool,
    group_id: GroupId,
    group_name: &str,
    member_name: &str,
) {
    notify_owner(
        bot,
        pool,
        group_id,
        texts::notify_preferences_changed(member_name, group_name),
    )
    .await;
}

/// Spawns independent deliveries; used after the group row (and its member
/// list) has already been captured and deleted.
pub fn group_deleted(bot: Bot, telegram_ids: Vec<TelegramId>, group_name: String) {
    for telegram_id in telegram_ids {
        let bot = bot.clone();
        let text = texts::notify_group_deleted(&group_name);
        tokio::spawn(async move { send(&bot, telegram_id, text).await });
    }
}

async fn notify_owner(bot: &Bot, pool: &SqlitePool, group_id: GroupId, text: String) {
    match groups::owner_telegram_id(pool, group_id).await {
        Ok(Some(telegram_id)) => send(bot, telegram_id, text).await,
        Ok(None) => tracing::warn!("owner notification skipped: group not found"),
        Err(_) => tracing::warn!("owner notification skipped: lookup failed"),
    }
}

async fn send(bot: &Bot, telegram_id: TelegramId, text: String) {
    let chat = ChatId(telegram_id.get());
    match tokio::time::timeout(DELIVERY_TIMEOUT, bot.send_message(chat, text)).await {
        Ok(Ok(_)) => {}
        Ok(Err(_)) => {
            tracing::warn!("Telegram notification delivery failed");
        }
        Err(_) => {
            tracing::warn!("Telegram notification delivery timed out");
        }
    }
}
