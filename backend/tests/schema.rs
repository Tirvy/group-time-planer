mod common;

use common::*;
use raid_scheduler_backend::{config::Config, db};

#[tokio::test]
async fn migrations_create_expected_schema_and_are_idempotent() {
    let test_db = test_db().await;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name
         FROM sqlite_master
         WHERE type = 'table'
           AND name NOT LIKE 'sqlite_%'
           AND name <> '_sqlx_migrations'
         ORDER BY name",
    )
    .fetch_all(&test_db.pool)
    .await
    .unwrap();

    assert_eq!(
        tables,
        vec![
            "group_members",
            "groups",
            "invitations",
            "users",
            "weekly_preferences"
        ]
    );

    let applied = count(&test_db.pool, "SELECT COUNT(*) FROM _sqlx_migrations").await;
    assert_eq!(applied, 1);

    // Running migrations again must be a no-op and must not touch data.
    seed_user(&test_db.pool, 900_001).await;
    db::migrate(&test_db.pool).await.unwrap();

    let applied_after = count(&test_db.pool, "SELECT COUNT(*) FROM _sqlx_migrations").await;
    assert_eq!(applied_after, 1);
    assert_eq!(count(&test_db.pool, "SELECT COUNT(*) FROM users").await, 1);
}

#[tokio::test]
async fn data_survives_reconnect_and_migration() {
    let test_db = test_db().await;
    let user = seed_user(&test_db.pool, 900_002).await;
    test_db.pool.close().await;

    let config = Config {
        database_path: test_db.path.clone(),
        ..Config::default()
    };
    let pool = db::connect(&config).await.unwrap();
    db::migrate(&pool).await.unwrap();

    let persisted: i64 = sqlx::query_scalar("SELECT id FROM users WHERE telegram_id = ?")
        .bind(900_002)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(persisted, user.get());
}

#[tokio::test]
async fn telegram_ids_are_unique() {
    let test_db = test_db().await;
    seed_user(&test_db.pool, 900_003).await;

    let now = now_ms();
    let duplicate = sqlx::query(
        "INSERT INTO users (telegram_id, first_name, created_at, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(900_003)
    .bind("Дубликат")
    .bind(now)
    .bind(now)
    .execute(&test_db.pool)
    .await;

    assert!(duplicate.is_err(), "telegram_id must be unique");
    assert_eq!(count(&test_db.pool, "SELECT COUNT(*) FROM users").await, 1);
}

#[tokio::test]
async fn group_name_constraints_are_enforced_by_the_database() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 900_004).await;
    let now = now_ms();

    for name in ["", "   ", &"я".repeat(65)] {
        let result = sqlx::query(
            "INSERT INTO groups (name, owner_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(name)
        .bind(owner.get())
        .bind(now)
        .bind(now)
        .execute(&test_db.pool)
        .await;
        assert!(result.is_err(), "name {name:?} must be rejected");
    }
}

#[tokio::test]
async fn groups_require_an_existing_owner() {
    let test_db = test_db().await;
    let now = now_ms();

    let result = sqlx::query(
        "INSERT INTO groups (name, owner_user_id, created_at, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind("Без владельца")
    .bind(999_999_i64)
    .bind(now)
    .bind(now)
    .execute(&test_db.pool)
    .await;

    assert!(result.is_err(), "owner_user_id must reference users");
}
