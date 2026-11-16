mod common;

use common::*;
use raid_scheduler_backend::{
    domain::{InvitationToken, MAX_GROUP_MEMBERS},
    services::{
        invitations::{self, InvitationError, JoinOutcome, RevokeOutcome},
        members, membership,
    },
};

async fn invite(
    pool: &sqlx::SqlitePool,
    owner: raid_scheduler_backend::domain::UserId,
    group: raid_scheduler_backend::domain::GroupId,
) -> invitations::CreatedInvitation {
    invitations::create(pool, owner, group, None)
        .await
        .expect("create invitation")
}

#[tokio::test]
async fn invitation_stores_only_a_hash_and_returns_the_raw_token_once() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_001).await;
    let group = seed_group(&test_db.pool, owner, "Хеширование").await;

    let created = invite(&test_db.pool, owner, group.id).await;

    let stored: String = sqlx::query_scalar("SELECT token_hash FROM invitations WHERE id = ?")
        .bind(created.id.get())
        .fetch_one(&test_db.pool)
        .await
        .unwrap();

    assert_eq!(stored, created.token.hash());
    assert_ne!(stored, created.token.expose());
    assert_eq!(stored.len(), 64);

    let raw_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE token_hash = ?")
        .bind(created.token.expose())
        .fetch_one(&test_db.pool)
        .await
        .unwrap();
    assert_eq!(raw_rows, 0, "raw token must never be stored");
}

#[tokio::test]
async fn only_owner_can_create_invitations_and_expiry_must_be_future() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_101).await;
    let member = seed_user(&test_db.pool, 10_102).await;
    let group = seed_group(&test_db.pool, owner, "Права").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    let error = invitations::create(&test_db.pool, member, group.id, None)
        .await
        .expect_err("members cannot invite");
    assert!(matches!(error, InvitationError::NotOwner(_)));

    let error = invitations::create(&test_db.pool, owner, group.id, Some(now_ms() - 1))
        .await
        .expect_err("past expiry must be rejected");
    assert!(matches!(error, InvitationError::InvalidExpiry));

    let created = invitations::create(&test_db.pool, owner, group.id, Some(now_ms() + 60_000))
        .await
        .unwrap();
    assert!(created.expires_at.is_some());
}

#[tokio::test]
async fn preview_is_read_only_and_reports_membership_capacity_and_owner() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_201).await;
    let member = seed_user(&test_db.pool, 10_202).await;
    let outsider = seed_user(&test_db.pool, 10_203).await;
    let group = seed_group(&test_db.pool, owner, "Предпросмотр").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();

    let created = invite(&test_db.pool, owner, group.id).await;
    let preview = invitations::preview(&test_db.pool, &created.token, outsider)
        .await
        .unwrap();

    assert_eq!(preview.group_name, "Предпросмотр");
    assert_eq!(preview.member_count, 2);
    assert!(!preview.is_full);
    assert!(!preview.is_member);
    assert!(preview.members[0].is_owner());
    assert_eq!(preview.members[0].user_id, owner);
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        2,
        "preview must not join anyone"
    );

    let member_preview = invitations::preview(&test_db.pool, &created.token, member)
        .await
        .unwrap();
    assert!(member_preview.is_member);
}

#[tokio::test]
async fn preview_rejects_invalid_expired_and_revoked_invitations() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_301).await;
    let outsider = seed_user(&test_db.pool, 10_302).await;
    let group = seed_group(&test_db.pool, owner, "Срок").await;

    let unknown = InvitationToken::from_raw(&"A".repeat(43)).unwrap();
    let error = invitations::preview(&test_db.pool, &unknown, outsider)
        .await
        .expect_err("unknown token must fail");
    assert!(matches!(error, InvitationError::InvalidToken));

    let expired = invite(&test_db.pool, owner, group.id).await;
    let past = now_ms();
    sqlx::query("UPDATE invitations SET created_at = ?, expires_at = ? WHERE id = ?")
        .bind(past - 2_000)
        .bind(past - 1_000)
        .bind(expired.id.get())
        .execute(&test_db.pool)
        .await
        .unwrap();
    let error = invitations::preview(&test_db.pool, &expired.token, outsider)
        .await
        .expect_err("expired token must fail");
    assert!(matches!(error, InvitationError::Expired));

    let revoked = invite(&test_db.pool, owner, group.id).await;
    invitations::revoke(&test_db.pool, owner, revoked.id)
        .await
        .unwrap();
    let error = invitations::preview(&test_db.pool, &revoked.token, outsider)
        .await
        .expect_err("revoked token must fail");
    assert!(matches!(error, InvitationError::Revoked));
}

#[tokio::test]
async fn join_is_idempotent_and_creates_a_pending_membership() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_401).await;
    let outsider = seed_user(&test_db.pool, 10_402).await;
    let group = seed_group(&test_db.pool, owner, "Вступление").await;
    let created = invite(&test_db.pool, owner, group.id).await;

    let outcome = invitations::join(&test_db.pool, &created.token, outsider)
        .await
        .unwrap();
    assert!(matches!(outcome, JoinOutcome::Joined { .. }));

    let (role, setup): (String, Option<i64>) = sqlx::query_as(
        "SELECT role, setup_completed_at FROM group_members WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.id.get())
    .bind(outsider.get())
    .fetch_one(&test_db.pool)
    .await
    .unwrap();
    assert_eq!(role, "member");
    assert!(setup.is_none(), "joined member starts pending");

    let outcome = invitations::join(&test_db.pool, &created.token, outsider)
        .await
        .unwrap();
    assert!(matches!(outcome, JoinOutcome::AlreadyMember { .. }));
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        2,
        "repeated confirmation must not duplicate membership"
    );
}

#[tokio::test]
async fn join_revalidates_capacity_and_invitation_state() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_501).await;
    let group = seed_group(&test_db.pool, owner, "Полная").await;
    for offset in 1..MAX_GROUP_MEMBERS {
        let member = seed_user(&test_db.pool, 10_550 + offset as i64).await;
        membership::add(&test_db.pool, group.id, member)
            .await
            .unwrap();
    }

    let created = invite(&test_db.pool, owner, group.id).await;
    let late = seed_user(&test_db.pool, 10_700).await;
    let error = invitations::join(&test_db.pool, &created.token, late)
        .await
        .expect_err("21st member must be rejected");
    assert!(matches!(error, InvitationError::GroupFull));
    assert_eq!(
        count(&test_db.pool, "SELECT COUNT(*) FROM group_members").await,
        MAX_GROUP_MEMBERS as i64
    );

    let expired_group = seed_group(&test_db.pool, owner, "Истекла").await;
    let expired = invite(&test_db.pool, owner, expired_group.id).await;
    let past = now_ms();
    sqlx::query("UPDATE invitations SET created_at = ?, expires_at = ? WHERE id = ?")
        .bind(past - 2_000)
        .bind(past - 1_000)
        .bind(expired.id.get())
        .execute(&test_db.pool)
        .await
        .unwrap();
    let error = invitations::join(&test_db.pool, &expired.token, late)
        .await
        .expect_err("expired invitation must not join");
    assert!(matches!(error, InvitationError::Expired));
}

#[tokio::test]
async fn revoke_is_owner_only_idempotent_and_blocks_join() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_801).await;
    let member = seed_user(&test_db.pool, 10_802).await;
    let outsider = seed_user(&test_db.pool, 10_803).await;
    let group = seed_group(&test_db.pool, owner, "Отзыв").await;
    membership::add(&test_db.pool, group.id, member)
        .await
        .unwrap();
    let created = invite(&test_db.pool, owner, group.id).await;

    let error = invitations::revoke(&test_db.pool, member, created.id)
        .await
        .expect_err("members cannot revoke");
    assert!(matches!(error, InvitationError::NotOwner(_)));

    let outcome = invitations::revoke(&test_db.pool, owner, created.id)
        .await
        .unwrap();
    assert!(matches!(outcome, RevokeOutcome::Revoked { .. }));

    let outcome = invitations::revoke(&test_db.pool, owner, created.id)
        .await
        .unwrap();
    assert!(matches!(outcome, RevokeOutcome::AlreadyRevoked { .. }));

    let error = invitations::join(&test_db.pool, &created.token, outsider)
        .await
        .expect_err("revoked invitation must not join");
    assert!(matches!(error, InvitationError::Revoked));
}

#[tokio::test]
async fn preview_and_join_reject_unknown_tokens() {
    let test_db = test_db().await;
    let owner = seed_user(&test_db.pool, 10_901).await;
    let outsider = seed_user(&test_db.pool, 10_902).await;
    let group = seed_group(&test_db.pool, owner, "Токены").await;
    let unknown = InvitationToken::from_raw(&"A".repeat(43)).unwrap();

    let error = invitations::preview(&test_db.pool, &unknown, outsider)
        .await
        .expect_err("preview must reject unknown token");
    assert!(matches!(error, InvitationError::InvalidToken));

    let error = invitations::join(&test_db.pool, &unknown, outsider)
        .await
        .expect_err("join must reject unknown token");
    assert!(matches!(error, InvitationError::InvalidToken));

    // Keep a valid group in the database to ensure the failure is about the token.
    assert_eq!(
        members::list_for_group(&test_db.pool, owner, group.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
