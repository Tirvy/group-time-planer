mod common;

use common::*;
use raid_scheduler_backend::services::membership;

#[tokio::test]
async fn leaving_a_group_removes_only_that_groups_preferences() {
    let test_db = test_db().await;
    let owner_a = seed_user(&test_db.pool, 7001).await;
    let owner_b = seed_user(&test_db.pool, 7002).await;
    let user = seed_user(&test_db.pool, 7003).await;
    let group_a = seed_group(&test_db.pool, owner_a, "Группа A").await;
    let group_b = seed_group(&test_db.pool, owner_b, "Группа B").await;

    membership::add(&test_db.pool, group_a.id, user)
        .await
        .unwrap();
    membership::add(&test_db.pool, group_b.id, user)
        .await
        .unwrap();

    insert_preference(&test_db.pool, group_a.id, user, 1, 600, 660)
        .await
        .unwrap();
    insert_preference(&test_db.pool, group_b.id, user, 1, 600, 660)
        .await
        .unwrap();
    insert_preference(&test_db.pool, group_b.id, user, 2, 1200, 1260)
        .await
        .unwrap();

    // Leaving group A is represented by removing that membership row.
    sqlx::query("DELETE FROM group_members WHERE group_id = ? AND user_id = ?")
        .bind(group_a.id.get())
        .bind(user.get())
        .execute(&test_db.pool)
        .await
        .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM weekly_preferences WHERE group_id = ? AND user_id = ?"
        )
        .bind(group_a.id.get())
        .bind(user.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap(),
        0,
        "group A preferences must be deleted with the membership"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM weekly_preferences WHERE group_id = ? AND user_id = ?"
        )
        .bind(group_b.id.get())
        .bind(user.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap(),
        2,
        "group B preferences must be untouched"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM group_members WHERE group_id = ? AND user_id = ?"
        )
        .bind(group_b.id.get())
        .bind(user.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM groups").await,
        2,
        "leaving must not delete the group"
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM users").await,
        3,
        "user profiles survive leaving a group"
    );
}

#[tokio::test]
async fn deleting_a_group_cascades_members_preferences_and_invitations() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 7101).await;
    let member = seed_user(&test_db.pool, 7102).await;
    let group = seed_group(&test_db.pool, owner, "Удаляемая группа").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    insert_preference(&test_db.pool, group.id, owner, 1, 600, 660)
        .await
        .unwrap();
    insert_preference(&test_db.pool, group.id, member, 2, 660, 720)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO invitations
             (group_id, token_hash, created_by_user_id, created_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(group.id.get())
    .bind("a".repeat(64))
    .bind(owner.get())
    .bind(now_ms())
    .execute(&test_db.pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(group.id.get())
        .execute(&test_db.pool)
        .await
        .unwrap();

    assert_eq!(count(&test_db.pool, "SELECT COUNT(*) FROM groups").await, 0);
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        0
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        0
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM invitations").await,
        0
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM users").await,
        2,
        "user profiles must survive group deletion"
    );
}

#[tokio::test]
async fn preferences_require_group_membership() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 7201).await;
    let outsider = seed_user(&test_db.pool, 7202).await;
    let group = seed_group(&test_db.pool, owner, "Закрытая группа").await;

    let result = insert_preference(&test_db.pool, group.id, outsider, 1, 600, 660).await;
    assert!(result.is_err(), "preferences cannot exist for a non-member");
}

#[tokio::test]
async fn overlapping_preferences_are_rejected_but_adjacent_and_other_days_are_allowed() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 7301).await;
    let group = seed_group(&test_db.pool, owner, "Пересечения").await;

    insert_preference(&test_db.pool, group.id, owner, 1, 600, 660)
        .await
        .expect("first interval");

    let overlap = insert_preference(&test_db.pool, group.id, owner, 1, 630, 690)
        .await
        .expect_err("overlapping interval must be rejected");
    assert!(
        overlap.to_string().contains("overlapping_intervals"),
        "unexpected error: {overlap}"
    );

    insert_preference(&test_db.pool, group.id, owner, 1, 660, 720)
        .await
        .expect("adjacent interval is allowed");
    insert_preference(&test_db.pool, group.id, owner, 2, 600, 660)
        .await
        .expect("same clock interval on another day is allowed");

    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        3
    );
}

#[tokio::test]
async fn preference_boundaries_and_grid_are_enforced() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 7401).await;
    let group = seed_group(&test_db.pool, owner, "Границы").await;

    let invalid = [
        (1_i64, 600_i64, 600_i64),
        (0, 600, 660),
        (8, 600, 660),
        (1, 615, 660),
        (1, 600, 615),
        (1, 1440, 1440),
        (1, -30, 0),
        (1, 600, 1441),
        (1, 600, 1470),
    ];

    for (weekday, start, end) in invalid {
        let result = insert_preference(&test_db.pool, group.id, owner, weekday, start, end).await;
        assert!(
            result.is_err(),
            "({weekday}, {start}, {end}) must be rejected"
        );
    }

    insert_preference(&test_db.pool, group.id, owner, 7, 0, 1440)
        .await
        .expect("full-day interval [00:00, 24:00) is valid");
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        1
    );
}
