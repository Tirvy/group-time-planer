mod common;

use common::*;
use raid_scheduler_backend::{
    domain::{PreferenceError, PreferenceInterval, Weekday},
    services::preferences::{self, PreferencesError},
};

fn interval(weekday: Weekday, start: u16, end: u16) -> PreferenceInterval {
    PreferenceInterval::new(weekday, start, end).unwrap()
}

#[tokio::test]
async fn pending_member_can_save_an_empty_calendar_and_become_completed() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_001).await;
    let group = seed_group(&test_db.pool, owner, "Пустой календарь").await;

    let initial = preferences::get_own(&test_db.pool, owner, group.id)
        .await
        .unwrap();
    assert!(initial.intervals.is_empty());
    assert!(initial.setup_state().is_pending());

    let saved = preferences::replace(&test_db.pool, owner, group.id, Vec::new())
        .await
        .unwrap();
    assert!(saved.intervals.is_empty());
    assert!(saved.setup_completed_at.is_some());
    assert!(!saved.changed, "empty stayed empty");

    let after = preferences::get_own(&test_db.pool, owner, group.id)
        .await
        .unwrap();
    assert!(after.intervals.is_empty());
    assert!(
        !after.setup_state().is_pending(),
        "completed-empty is not pending"
    );
}

#[tokio::test]
async fn intervals_are_stored_sorted_and_replaced_atomically() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_101).await;
    let group = seed_group(&test_db.pool, owner, "Замена").await;

    let saved = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![
            interval(Weekday::Wednesday, 660, 720),
            interval(Weekday::Monday, 600, 660),
        ],
    )
    .await
    .unwrap();

    assert!(saved.changed);
    assert_eq!(saved.intervals.len(), 2);
    assert_eq!(saved.intervals[0].weekday(), Weekday::Monday);
    assert_eq!(saved.intervals[1].weekday(), Weekday::Wednesday);
    assert!(saved.setup_completed_at.is_some());

    let stored: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT weekday, start_minute, end_minute
         FROM weekly_preferences WHERE group_id = ? AND user_id = ?
         ORDER BY weekday, start_minute",
    )
    .bind(group.id.get())
    .bind(owner.get())
    .fetch_all(&test_db.pool)
    .await
    .unwrap();
    assert_eq!(
        stored,
        vec![(1, 600, 660), (3, 660, 720)],
        "rows are sorted and stored"
    );

    let second = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![interval(Weekday::Friday, 1200, 1260)],
    )
    .await
    .unwrap();

    assert!(second.changed);
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        1,
        "old intervals are fully replaced"
    );

    let unchanged = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![interval(Weekday::Friday, 1200, 1260)],
    )
    .await
    .unwrap();
    assert!(!unchanged.changed);
}

#[tokio::test]
async fn invalid_or_overlapping_intervals_do_not_touch_storage() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_201).await;
    let group = seed_group(&test_db.pool, owner, "Валидация").await;

    let error = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![
            interval(Weekday::Monday, 600, 660),
            interval(Weekday::Monday, 630, 690),
        ],
    )
    .await
    .expect_err("overlap must be rejected");
    assert!(matches!(
        error,
        PreferencesError::Validation(PreferenceError::Overlap(Weekday::Monday))
    ));

    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        0
    );
    let state: Option<i64> = sqlx::query_scalar(
        "SELECT setup_completed_at FROM group_members WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.id.get())
    .bind(owner.get())
    .fetch_one(&test_db.pool)
    .await
    .unwrap();
    assert!(state.is_none(), "failed save must not mark setup complete");
}

#[tokio::test]
async fn setup_completion_time_is_preserved_across_saves() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_301).await;
    let group = seed_group(&test_db.pool, owner, "Время настройки").await;

    let first = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![interval(Weekday::Monday, 600, 660)],
    )
    .await
    .unwrap();
    let completed_at = first.setup_completed_at.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let second = preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![interval(Weekday::Tuesday, 690, 750)],
    )
    .await
    .unwrap();

    assert_eq!(second.setup_completed_at, Some(completed_at));
}

#[tokio::test]
async fn only_members_can_read_or_replace_preferences() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_401).await;
    let outsider = seed_user(&test_db.pool, 30_402).await;
    let group = seed_group(&test_db.pool, owner, "Доступ").await;

    let error = preferences::get_own(&test_db.pool, outsider, group.id)
        .await
        .expect_err("outsider cannot read");
    assert!(matches!(error, PreferencesError::NotAMember(_)));

    let error = preferences::replace(&test_db.pool, outsider, group.id, Vec::new())
        .await
        .expect_err("outsider cannot write");
    assert!(matches!(error, PreferencesError::NotAMember(_)));
}

#[tokio::test]
async fn clearing_intervals_keeps_setup_completed() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 30_501).await;
    let group = seed_group(&test_db.pool, owner, "Очистка").await;

    preferences::replace(
        &test_db.pool,
        owner,
        group.id,
        vec![interval(Weekday::Sunday, 0, 1440)],
    )
    .await
    .unwrap();

    let cleared = preferences::replace(&test_db.pool, owner, group.id, Vec::new())
        .await
        .unwrap();
    assert!(cleared.changed);
    assert!(cleared.intervals.is_empty());
    assert!(cleared.setup_completed_at.is_some());

    let after = preferences::get_own(&test_db.pool, owner, group.id)
        .await
        .unwrap();
    assert!(after.intervals.is_empty());
    assert!(!after.setup_state().is_pending());
}
