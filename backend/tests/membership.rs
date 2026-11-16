mod common;

use common::*;
use raid_scheduler_backend::{
    domain::{GroupId, MAX_GROUP_MEMBERS, MEMBER_COLORS, UserId},
    services::membership::{self, MembershipError},
};

#[tokio::test]
async fn creator_becomes_owner_and_first_pending_member() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 1001).await;
    let group = seed_group(&test_db.pool, owner, "Мифик+ по средам").await;

    assert_eq!(group.owner_user_id, owner);

    let row: (String, String, Option<i64>) = sqlx::query_as(
        "SELECT role, color, setup_completed_at
         FROM group_members
         WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.id.get())
    .bind(owner.get())
    .fetch_one(&test_db.pool)
    .await
    .unwrap();

    assert_eq!(row.0, "owner");
    assert_eq!(row.1, MEMBER_COLORS[0]);
    assert!(
        row.2.is_none(),
        "a new member starts in setup_pending until preferences are saved"
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        1
    );
}

#[tokio::test]
async fn membership_is_unique_per_group_and_user() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 2001).await;
    let group = seed_group(&test_db.pool, owner, "Группа").await;
    let member = seed_user(&test_db.pool, 2002).await;

    membership::add(&test_db.pool, group.id, member)
        .await
        .expect("first join succeeds");

    let error = membership::add(&test_db.pool, group.id, member)
        .await
        .expect_err("second join must be rejected");
    assert!(matches!(
        error,
        MembershipError::AlreadyMember { group_id, user_id }
            if group_id == group.id && user_id == member
    ));

    let now = now_ms();
    let duplicate = sqlx::query(
        "INSERT INTO group_members
             (group_id, user_id, role, color, setup_completed_at, joined_at)
         VALUES (?, ?, 'member', '#000000', NULL, ?)",
    )
    .bind(group.id.get())
    .bind(member.get())
    .bind(now)
    .execute(&test_db.pool)
    .await;
    assert!(
        duplicate.is_err(),
        "the (group_id, user_id) primary key must reject duplicates"
    );

    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        2
    );
}

#[tokio::test]
async fn group_capacity_is_twenty_including_owner() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 3000).await;
    let group = seed_group(&test_db.pool, owner, "Большая группа").await;

    for offset in 1..MAX_GROUP_MEMBERS {
        let user = seed_user(&test_db.pool, 3000 + offset as i64).await;
        membership::add(&test_db.pool, group.id, user)
            .await
            .expect("join below capacity");
    }

    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        MAX_GROUP_MEMBERS as i64
    );

    // Owner plus 19 members use the whole palette exactly once.
    let mut assigned: Vec<String> =
        sqlx::query_scalar("SELECT color FROM group_members WHERE group_id = ?")
            .bind(group.id.get())
            .fetch_all(&test_db.pool)
            .await
            .unwrap();
    assigned.sort();
    let mut expected: Vec<String> = MEMBER_COLORS.iter().map(|c| (*c).to_owned()).collect();
    expected.sort();
    assert_eq!(assigned, expected);

    let extra = seed_user(&test_db.pool, 4000).await;
    let error = membership::add(&test_db.pool, group.id, extra)
        .await
        .expect_err("21st member must be rejected");
    assert!(matches!(
        error,
        MembershipError::GroupFull {
            group_id,
            max: MAX_GROUP_MEMBERS,
        } if group_id == group.id
    ));

    // Defense in depth: direct SQL cannot bypass the cap either.
    let now = now_ms();
    let direct = sqlx::query(
        "INSERT INTO group_members
             (group_id, user_id, role, color, setup_completed_at, joined_at)
         VALUES (?, ?, 'member', '#000000', NULL, ?)",
    )
    .bind(group.id.get())
    .bind(extra.get())
    .bind(now)
    .execute(&test_db.pool)
    .await;

    let message = direct
        .expect_err("trigger must reject the 21st member")
        .to_string();
    assert!(
        message.contains("group_full"),
        "unexpected error: {message}"
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        MAX_GROUP_MEMBERS as i64
    );
}

#[tokio::test]
async fn only_one_owner_membership_is_allowed_per_group() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 5001).await;
    let group = seed_group(&test_db.pool, owner, "Один владелец").await;
    let other = seed_user(&test_db.pool, 5002).await;
    let now = now_ms();

    let second_owner = sqlx::query(
        "INSERT INTO group_members
             (group_id, user_id, role, color, setup_completed_at, joined_at)
         VALUES (?, ?, 'owner', '#000000', NULL, ?)",
    )
    .bind(group.id.get())
    .bind(other.get())
    .bind(now)
    .execute(&test_db.pool)
    .await;

    assert!(
        second_owner.is_err(),
        "a group must have at most one owner membership"
    );
}

#[tokio::test]
async fn concurrent_joins_cannot_exceed_capacity() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 8000).await;
    let group = seed_group(&test_db.pool, owner, "Гонка за места").await;

    // Owner plus 18 members: one free seat remains.
    for offset in 1..(MAX_GROUP_MEMBERS - 1) {
        let user = seed_user(&test_db.pool, 8000 + offset as i64).await;
        membership::add(&test_db.pool, group.id, user)
            .await
            .expect("join below capacity");
    }

    let contender_a = seed_user(&test_db.pool, 8101).await;
    let contender_b = seed_user(&test_db.pool, 8102).await;

    let (first, second) = tokio::join!(
        membership::add(&test_db.pool, group.id, contender_a),
        membership::add(&test_db.pool, group.id, contender_b),
    );

    let results = [first, second];
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "exactly one concurrent join may take the last seat"
    );
    for result in results.into_iter().filter_map(Result::err) {
        assert!(
            matches!(result, MembershipError::GroupFull { .. }),
            "unexpected error: {result}"
        );
    }
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        MAX_GROUP_MEMBERS as i64
    );
}

#[tokio::test]
async fn joins_validate_group_and_user_existence() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 6001).await;
    let group = seed_group(&test_db.pool, owner, "Проверки").await;
    let user = seed_user(&test_db.pool, 6002).await;

    let missing_group = GroupId::new(999_999).unwrap();
    let error = membership::add(&test_db.pool, missing_group, user)
        .await
        .expect_err("unknown group must be rejected");
    assert!(matches!(
        error,
        MembershipError::GroupNotFound(id) if id == missing_group
    ));

    let missing_user = UserId::new(999_999).unwrap();
    let error = membership::add(&test_db.pool, group.id, missing_user)
        .await
        .expect_err("unknown user must be rejected");
    assert!(matches!(
        error,
        MembershipError::UserNotFound(id) if id == missing_user
    ));
}
