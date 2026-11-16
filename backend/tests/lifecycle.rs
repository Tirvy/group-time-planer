mod common;

use common::*;
use raid_scheduler_backend::{
    domain::{GroupId, GroupName},
    services::{
        groups::{self, GroupError},
        invitations,
        members::{self, MembersError},
        membership::{self, LeaveError},
    },
};

#[tokio::test]
async fn member_can_leave_and_only_this_groups_preferences_are_removed() {
    let test_db = test_db().await;
    let owner_a = seed_user(&test_db.pool, 20_001).await;
    let owner_b = seed_user(&test_db.pool, 20_002).await;
    let user = seed_user(&test_db.pool, 20_003).await;
    let group_a = seed_group(&test_db.pool, owner_a, "Выход A").await;
    let group_b = seed_group(&test_db.pool, owner_b, "Выход B").await;

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

    let left = membership::leave(&test_db.pool, group_a.id, user)
        .await
        .unwrap();
    assert_eq!(left.group_name, "Выход A");

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM weekly_preferences WHERE group_id = ? AND user_id = ?"
        )
        .bind(group_a.id.get())
        .bind(user.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap(),
        0
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
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM group_members WHERE group_id = ? AND user_id = ?"
        )
        .bind(group_a.id.get())
        .bind(user.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn owner_cannot_leave_and_non_member_cannot_leave() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_101).await;
    let outsider = seed_user(&test_db.pool, 20_102).await;
    let group = seed_group(&test_db.pool, owner, "Владелец остаётся").await;

    let error = membership::leave(&test_db.pool, group.id, owner)
        .await
        .expect_err("owner must not leave");
    assert!(matches!(error, LeaveError::OwnerCannotLeave(_)));

    let error = membership::leave(&test_db.pool, group.id, outsider)
        .await
        .expect_err("non-member must not leave");
    assert!(matches!(error, LeaveError::NotAMember(_)));

    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        1
    );
}

#[tokio::test]
async fn ownership_transfer_keeps_previous_owner_as_member() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_201).await;
    let target = seed_user(&test_db.pool, 20_202).await;
    let other = seed_user(&test_db.pool, 20_203).await;
    let group = seed_group(&test_db.pool, owner, "Передача").await;
    membership::add(&test_db.pool, group.id, target)
        .await
        .unwrap();
    membership::add(&test_db.pool, group.id, other)
        .await
        .unwrap();

    let transfer = groups::transfer_ownership(&test_db.pool, owner, group.id, target)
        .await
        .unwrap();
    assert_eq!(transfer.new_owner, target);

    let stored_owner: i64 = sqlx::query_scalar("SELECT owner_user_id FROM groups WHERE id = ?")
        .bind(group.id.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap();
    assert_eq!(stored_owner, target.get());

    let roles: Vec<(i64, String)> = sqlx::query_as(
        "SELECT user_id, role FROM group_members WHERE group_id = ? ORDER BY user_id",
    )
    .bind(group.id.get())
    .fetch_all(&test_db.pool)
    .await
    .unwrap();

    assert_eq!(
        roles,
        vec![
            (owner.get(), "member".to_owned()),
            (target.get(), "owner".to_owned()),
            (other.get(), "member".to_owned()),
        ]
    );
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        3,
        "previous owner remains a member"
    );

    let owner_list: Vec<(i64, String)> = sqlx::query_as(
        "SELECT user_id, role FROM group_members WHERE group_id = ? AND role = 'owner'",
    )
    .bind(group.id.get())
    .fetch_all(&test_db.pool)
    .await
    .unwrap();
    assert_eq!(owner_list.len(), 1);
}

#[tokio::test]
async fn transfer_rejects_non_owner_outsider_and_self() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_301).await;
    let member = seed_user(&test_db.pool, 20_302).await;
    let outsider = seed_user(&test_db.pool, 20_303).await;
    let group = seed_group(&test_db.pool, owner, "Проверки передачи").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    let error = groups::transfer_ownership(&test_db.pool, member, group.id, outsider)
        .await
        .expect_err("non-owner cannot transfer");
    assert!(matches!(error, GroupError::NotOwner(_)));

    let error = groups::transfer_ownership(&test_db.pool, owner, group.id, outsider)
        .await
        .expect_err("target must be a member");
    assert!(matches!(error, GroupError::TargetNotMember(_)));

    let error = groups::transfer_ownership(&test_db.pool, owner, group.id, owner)
        .await
        .expect_err("self-transfer must fail");
    assert!(matches!(error, GroupError::InvalidTransferTarget));
}

#[tokio::test]
async fn group_deletion_captures_members_and_cascades_all_group_data() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_401).await;
    let member = seed_user(&test_db.pool, 20_402).await;
    let group = seed_group(&test_db.pool, owner, "Удаление").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();
    insert_preference(&test_db.pool, group.id, owner, 1, 600, 660)
        .await
        .unwrap();
    invitations::create(&test_db.pool, owner, group.id, None)
        .await
        .unwrap();

    let error = groups::delete_group(&test_db.pool, member, group.id)
        .await
        .expect_err("only owner can delete");
    assert!(matches!(error, GroupError::NotOwner(_)));

    let deleted = groups::delete_group(&test_db.pool, owner, group.id)
        .await
        .unwrap();
    assert_eq!(deleted.group_name, "Удаление");

    let mut captured: Vec<i64> = deleted
        .member_telegram_ids
        .iter()
        .map(|id| id.get())
        .collect();
    captured.sort_unstable();
    assert_eq!(captured, vec![20_401, 20_402]);

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
        "user profiles survive group deletion"
    );
}

#[tokio::test]
async fn rename_is_owner_only_and_persists() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_501).await;
    let member = seed_user(&test_db.pool, 20_502).await;
    let group = seed_group(&test_db.pool, owner, "Старое имя").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    groups::rename(
        &test_db.pool,
        owner,
        group.id,
        &GroupName::parse("Новое имя").unwrap(),
    )
    .await
    .unwrap();

    let stored: String = sqlx::query_scalar("SELECT name FROM groups WHERE id = ?")
        .bind(group.id.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap();
    assert_eq!(stored, "Новое имя");

    let error = groups::rename(
        &test_db.pool,
        member,
        group.id,
        &GroupName::parse("Чужое имя").unwrap(),
    )
    .await
    .expect_err("members cannot rename");
    assert!(matches!(error, GroupError::NotOwner(_)));

    let error = groups::rename(
        &test_db.pool,
        owner,
        GroupId::new(999_999).unwrap(),
        &GroupName::parse("Нет группы").unwrap(),
    )
    .await
    .expect_err("unknown group must fail");
    assert!(matches!(error, GroupError::GroupNotFound(_)));
}

#[tokio::test]
async fn members_list_is_owner_first_and_requires_membership() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 20_601).await;
    let member = seed_user(&test_db.pool, 20_602).await;
    let outsider = seed_user(&test_db.pool, 20_603).await;
    let group = seed_group(&test_db.pool, owner, "Список").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    sqlx::query(
        "UPDATE group_members SET setup_completed_at = ? WHERE group_id = ? AND user_id = ?",
    )
    .bind(now_ms())
    .bind(group.id.get())
    .bind(member.get())
    .execute(&test_db.pool)
    .await
    .unwrap();

    let list = members::list_for_group(&test_db.pool, member, group.id)
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].is_owner());
    assert_eq!(list[0].user_id, owner);
    assert_eq!(list[1].user_id, member);
    assert!(!list[1].setup_state().is_pending());

    let error = members::list_for_group(&test_db.pool, outsider, group.id)
        .await
        .expect_err("outsider must not list members");
    assert!(matches!(error, MembersError::NotAMember(_)));
}
