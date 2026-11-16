#![allow(dead_code)]

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use raid_scheduler_backend::{
    config::Config,
    db,
    domain::{GroupId, GroupName, UserId},
    services::groups::{self, Group},
};
use sqlx::SqlitePool;
use tempfile::TempDir;

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
