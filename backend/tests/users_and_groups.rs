mod common;

use common::*;
use raid_scheduler_backend::{
    domain::{SetupState, TelegramId},
    services::{
        groups::{self, GroupError},
        membership,
        users::{self, TelegramProfile},
    },
};

fn profile(telegram_id: i64, username: &str, first: &str, last: Option<&str>) -> TelegramProfile {
    TelegramProfile::new(
        TelegramId::new(telegram_id).unwrap(),
        Some(username.to_owned()),
        Some(first.to_owned()),
        last.map(str::to_owned),
    )
}

#[tokio::test]
async fn upsert_creates_user_then_refreshes_mutable_profile_fields() {
    let test_db = test_db().await;

    let first = users::upsert(&test_db.pool, &profile(9101, "old_name", "Иван", None))
        .await
        .unwrap();
    assert_eq!(first.username.as_deref(), Some("old_name"));
    assert_eq!(first.first_name.as_deref(), Some("Иван"));
    assert_eq!(first.last_name, None);
    assert!(first.timezone.is_none());

    let updated = users::upsert(
        &test_db.pool,
        &profile(9101, "new_name", "Пётр", Some("Сидоров")),
    )
    .await
    .unwrap();

    assert_eq!(updated.id, first.id, "identity is derived from telegram_id");
    assert_eq!(updated.username.as_deref(), Some("new_name"));
    assert_eq!(updated.first_name.as_deref(), Some("Пётр"));
    assert_eq!(updated.last_name.as_deref(), Some("Сидоров"));
    assert_eq!(count(&test_db.pool, "SELECT COUNT(*) FROM users").await, 1);
}

#[tokio::test]
async fn upsert_does_not_clear_the_selected_timezone() {
    let test_db = test_db().await;
    let user = users::upsert(&test_db.pool, &profile(9102, "tz_user", "Анна", None))
        .await
        .unwrap();

    sqlx::query("UPDATE users SET timezone = ? WHERE id = ?")
        .bind("Asia/Tbilisi")
        .bind(user.id.get())
        .execute(&test_db.pool)
        .await
        .unwrap();

    let refreshed = users::upsert(&test_db.pool, &profile(9102, "tz_user2", "Анна", None))
        .await
        .unwrap();

    assert_eq!(refreshed.timezone.as_deref(), Some("Asia/Tbilisi"));
}

#[tokio::test]
async fn list_for_user_returns_only_memberships_with_owner_first() {
    let test_db = test_db().await;
    let user = seed_user(&test_db.pool, 9201).await;
    let other_owner = seed_user(&test_db.pool, 9202).await;

    // Two groups owned by the user, one group joined as a member, one unrelated.
    let owned_a = seed_group(&test_db.pool, user, "Своя A").await;
    let owned_b = seed_group(&test_db.pool, user, "Своя B").await;
    let joined = seed_group(&test_db.pool, other_owner, "Чужая").await;
    membership::add(&test_db.pool, joined.id, user)
        .await
        .unwrap();
    let _unrelated = seed_group(&test_db.pool, other_owner, "Мимо").await;

    // Mark setup completed in the joined group to exercise the state mapping.
    sqlx::query(
        "UPDATE group_members SET setup_completed_at = ? WHERE group_id = ? AND user_id = ?",
    )
    .bind(now_ms())
    .bind(joined.id.get())
    .bind(user.get())
    .execute(&test_db.pool)
    .await
    .unwrap();

    let summaries = groups::list_for_user(&test_db.pool, user).await.unwrap();
    assert_eq!(summaries.len(), 3, "unrelated group must not be listed");

    assert!(summaries[0].is_owner());
    assert!(summaries[1].is_owner());
    assert_scoped(&summaries, owned_a.id, 1, SetupState::Pending);
    assert_scoped(&summaries, owned_b.id, 1, SetupState::Pending);
    assert_scoped(&summaries, joined.id, 2, SetupState::Completed);
}

fn assert_scoped(
    summaries: &[groups::GroupSummary],
    group_id: raid_scheduler_backend::domain::GroupId,
    member_count: i64,
    state: SetupState,
) {
    let summary = summaries
        .iter()
        .find(|summary| summary.id == group_id)
        .expect("group in list");
    assert_eq!(summary.member_count, member_count);
    assert_eq!(summary.setup_state(), state);
}

#[tokio::test]
async fn get_for_user_enforces_current_membership() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 9301).await;
    let member = seed_user(&test_db.pool, 9302).await;
    let outsider = seed_user(&test_db.pool, 9303).await;
    let group = seed_group(&test_db.pool, owner, "Проверка доступа").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    let summary = groups::get_for_user(&test_db.pool, member, group.id)
        .await
        .unwrap();
    assert_eq!(summary.id, group.id);
    assert_eq!(summary.member_count, 2);
    assert!(!summary.is_owner());

    let error = groups::get_for_user(&test_db.pool, outsider, group.id)
        .await
        .expect_err("non-members must not see the group");
    assert!(matches!(error, GroupError::NotAMember(id) if id == group.id));

    let missing = raid_scheduler_backend::domain::GroupId::new(999_999).unwrap();
    let error = groups::get_for_user(&test_db.pool, owner, missing)
        .await
        .expect_err("unknown group must not be exposed");
    assert!(matches!(error, GroupError::NotAMember(id) if id == missing));
}

#[tokio::test]
async fn list_for_user_is_empty_without_memberships() {
    let test_db = test_db().await;
    let user = seed_user(&test_db.pool, 9401).await;
    let summaries = groups::list_for_user(&test_db.pool, user).await.unwrap();
    assert!(summaries.is_empty());
}
