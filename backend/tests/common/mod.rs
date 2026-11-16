#![allow(dead_code)]

use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use raid_scheduler_backend::{
    api::{self, AppState},
    auth::AuthSettings,
    config::{Config, SecretString},
    db,
    domain::{GroupId, GroupName, UserId},
    services::groups::{self, Group},
};
use sha2::Sha256;
use sqlx::SqlitePool;
use tempfile::TempDir;
use url::form_urlencoded;

pub const TEST_BOT_TOKEN: &str = "123456:TEST-BOT-TOKEN";

pub struct TestDb {
    pub pool: SqlitePool,
    pub path: PathBuf,
    _dir: TempDir,
}

pub async fn test_db() -> TestDb {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path().join("data/app.db");
    let config = Config {
        database_path: path.clone(),
        ..Config::default()
    };
    let pool = db::connect(&config).await.expect("open test database");
    db::migrate(&pool).await.expect("apply migrations");
    TestDb {
        pool,
        path,
        _dir: dir,
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_millis() as i64
}

pub async fn seed_user(pool: &SqlitePool, telegram_id: i64) -> UserId {
    let now = now_ms();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO users (telegram_id, first_name, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         RETURNING id",
    )
    .bind(telegram_id)
    .bind("Тестовый участник")
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .expect("insert user");
    UserId::new(id).expect("positive user id")
}

pub async fn seed_group(pool: &SqlitePool, owner: UserId, name: &str) -> Group {
    let name = GroupName::parse(name).expect("valid group name");
    groups::create_with_owner(pool, &name, owner)
        .await
        .expect("create group")
}

pub async fn insert_preference(
    pool: &SqlitePool,
    group: GroupId,
    user: UserId,
    weekday: i64,
    start_minute: i64,
    end_minute: i64,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO weekly_preferences
             (group_id, user_id, weekday, start_minute, end_minute)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(group.get())
    .bind(user.get())
    .bind(weekday)
    .bind(start_minute)
    .bind(end_minute)
    .execute(pool)
    .await
}

pub async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .expect("count")
}

pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_secs() as i64
}

pub fn test_auth_settings() -> AuthSettings {
    AuthSettings {
        bot_token: Some(SecretString::new(TEST_BOT_TOKEN)),
        max_age: Duration::from_secs(3600),
    }
}

pub fn test_router(pool: &SqlitePool, auth: AuthSettings) -> axum::Router {
    api::router(AppState {
        db: pool.clone(),
        auth,
    })
}

pub async fn test_api() -> (axum::Router, TestDb) {
    let test_db = test_db().await;
    let router = test_router(&test_db.pool, test_auth_settings());
    (router, test_db)
}

/// Builds signed Telegram Mini App initData for API tests.
pub fn sign_init_data(token: &str, telegram_id: i64, auth_date: i64, first_name: &str) -> String {
    let user = format!(
        r#"{{"id":{telegram_id},"first_name":"{first_name}","last_name":"Тестов","username":"tester"}}"#
    );
    let fields = [
        ("auth_date", auth_date.to_string()),
        ("query_id", "AAH-test-query".to_owned()),
        ("user", user),
    ];

    let mut sorted: Vec<(&str, &str)> = fields
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    let data_check_string = sorted
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut key_mac = Hmac::<Sha256>::new_from_slice(b"WebAppData").unwrap();
    key_mac.update(token.as_bytes());
    let secret_key = key_mac.finalize().into_bytes();

    let mut mac = Hmac::<Sha256>::new_from_slice(&secret_key).unwrap();
    mac.update(data_check_string.as_bytes());
    let hash = hex::encode(mac.finalize().into_bytes());

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in &fields {
        serializer.append_pair(key, value);
    }
    serializer.append_pair("hash", &hash);
    serializer.finish()
}

pub fn auth_header(init_data: &str) -> String {
    format!("tma {init_data}")
}
