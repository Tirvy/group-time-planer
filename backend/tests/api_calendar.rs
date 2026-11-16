mod common;

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{
        Method, Request, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use chrono::{DateTime, Utc};
use common::*;
use http_body_util::BodyExt;
use raid_scheduler_backend::{
    domain::{GroupId, IanaTimezone, PreferenceInterval, UserId, Weekday},
    services::{clock::FixedClock, membership, preferences, users},
};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    auth: Option<String>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(auth) = auth {
        builder = builder.header(AUTHORIZATION, auth);
    }
    let body = if let Some(body) = body {
        builder = builder.header(CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };

    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn internal_user_id(pool: &sqlx::SqlitePool, telegram_id: i64) -> UserId {
    let raw: i64 = sqlx::query_scalar("SELECT id FROM users WHERE telegram_id = ?")
        .bind(telegram_id)
        .fetch_one(pool)
        .await
        .unwrap();
    UserId::new(raw).unwrap()
}

async fn authenticate(app: &Router, telegram_id: i64, name: &str) -> String {
    let init_data = sign_init_data(TEST_BOT_TOKEN, telegram_id, now_secs(), name);
    let (status, body) = call(
        app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    init_data
}

async fn set_timezone(app: &Router, init_data: &str, timezone: &str) {
    let (status, body) = call(
        app,
        Method::PUT,
        "/api/me/timezone",
        Some(auth_header(init_data)),
        Some(json!({ "timezone": timezone })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

struct CalendarFixture {
    app: Router,
    db: TestDb,
    group: GroupId,
    owner: UserId,
    tbilisi_member: UserId,
    berlin_member: UserId,
    pending_member: UserId,
    owner_init: String,
}

async fn calendar_fixture() -> CalendarFixture {
    let test_db = test_db().await;
    let fixed_now = DateTime::parse_from_rfc3339("2026-11-16T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let app = test_router_with_clock(
        &test_db.pool,
        test_auth_settings(),
        Arc::new(FixedClock::new(fixed_now)),
    );

    let owner_init = authenticate(&app, 42_001, "Владелец").await;
    let owner = internal_user_id(&test_db.pool, 42_001).await;
    let group = seed_group(&test_db.pool, owner, "Часовые пояса").await;
    users::update_timezone(
        &test_db.pool,
        owner,
        IanaTimezone::parse("Europe/Berlin").unwrap(),
    )
    .await
    .unwrap();

    let tbilisi_member = seed_user(&test_db.pool, 42_002).await;
    membership::add(&test_db.pool, group.id, tbilisi_member)
        .await
        .unwrap();
    users::update_timezone(
        &test_db.pool,
        tbilisi_member,
        IanaTimezone::parse("Asia/Tbilisi").unwrap(),
    )
    .await
    .unwrap();
    preferences::replace(
        &test_db.pool,
        tbilisi_member,
        group.id,
        vec![PreferenceInterval::new(Weekday::Monday, 21 * 60, 22 * 60).unwrap()],
    )
    .await
    .unwrap();

    let berlin_member = seed_user(&test_db.pool, 42_003).await;
    membership::add(&test_db.pool, group.id, berlin_member)
        .await
        .unwrap();
    users::update_timezone(
        &test_db.pool,
        berlin_member,
        IanaTimezone::parse("Europe/Berlin").unwrap(),
    )
    .await
    .unwrap();
    preferences::replace(
        &test_db.pool,
        berlin_member,
        group.id,
        vec![PreferenceInterval::new(Weekday::Monday, 18 * 60, 19 * 60).unwrap()],
    )
    .await
    .unwrap();

    let pending_member = seed_user(&test_db.pool, 42_004).await;
    membership::add(&test_db.pool, group.id, pending_member)
        .await
        .unwrap();
    users::update_timezone(
        &test_db.pool,
        pending_member,
        IanaTimezone::parse("Asia/Tbilisi").unwrap(),
    )
    .await
    .unwrap();

    CalendarFixture {
        app,
        db: test_db,
        group: group.id,
        owner,
        tbilisi_member,
        berlin_member,
        pending_member,
        owner_init,
    }
}

fn find_member(body: &Value, user_id: UserId) -> &Value {
    let expected = user_id.get().to_string();
    body["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["user_id"].as_str() == Some(expected.as_str()))
        .expect("member in calendar")
}

#[tokio::test]
async fn combined_calendar_converts_every_member_into_viewer_timezone() {
    let fixture = calendar_fixture().await;
    let path = format!("/api/groups/{}/calendar", fixture.group.get());

    let (status, body) = call(
        &fixture.app,
        Method::GET,
        &path,
        Some(auth_header(&fixture.owner_init)),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["viewer_timezone"], "Europe/Berlin");
    assert_eq!(body["reference_week"]["start_date"], "2026-11-16");
    assert_eq!(body["reference_week"]["end_date"], "2026-11-22");
    assert_eq!(body["group"]["name"], "Часовые пояса");

    let tbilisi = find_member(&body, fixture.tbilisi_member);
    let berlin = find_member(&body, fixture.berlin_member);
    let pending = find_member(&body, fixture.pending_member);

    assert_eq!(tbilisi["setup_state"], "completed");
    assert_eq!(berlin["setup_state"], "completed");
    assert_eq!(pending["setup_state"], "pending");
    assert_eq!(pending["segments"].as_array().unwrap().len(), 0);

    for member in [tbilisi, berlin] {
        let segments = member["segments"].as_array().unwrap();
        assert_eq!(segments.len(), 1, "{member}");
        assert_eq!(segments[0]["day"], 0);
        assert_eq!(segments[0]["start_minute"], 18 * 60);
        assert_eq!(segments[0]["end_minute"], 19 * 60);
        assert_eq!(segments[0]["start"], "2026-11-16T18:00:00+01:00");
        assert_eq!(segments[0]["end"], "2026-11-16T19:00:00+01:00");
    }

    assert_eq!(
        body["members"].as_array().unwrap().len(),
        4,
        "owner and all members are included"
    );
    assert_eq!(
        body["members"][0]["user_id"].as_str(),
        Some(fixture.owner.get().to_string().as_str()),
        "owner is first"
    );
}

#[tokio::test]
async fn individual_calendar_is_available_to_any_member() {
    let fixture = calendar_fixture().await;
    let path = format!(
        "/api/groups/{}/members/{}/calendar",
        fixture.group.get(),
        fixture.tbilisi_member.get()
    );

    let (status, body) = call(
        &fixture.app,
        Method::GET,
        &path,
        Some(auth_header(&fixture.owner_init)),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["member"]["user_id"].as_str(),
        Some(fixture.tbilisi_member.get().to_string().as_str())
    );
    assert_eq!(body["member"]["setup_state"], "completed");
    assert_eq!(body["member"]["segments"][0]["start_minute"], 18 * 60);

    let outsider = seed_user(&fixture.db.pool, 42_099).await;
    let missing_path = format!(
        "/api/groups/{}/members/{}/calendar",
        fixture.group.get(),
        outsider.get()
    );
    let (status, body) = call(
        &fixture.app,
        Method::GET,
        &missing_path,
        Some(auth_header(&fixture.owner_init)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "MEMBER_NOT_FOUND");
}

#[tokio::test]
async fn calendars_require_membership_and_a_timezone() {
    let fixture = calendar_fixture().await;
    let outsider_init = authenticate(&fixture.app, 42_101, "Чужой").await;
    set_timezone(&fixture.app, &outsider_init, "Europe/Berlin").await;
    let path = format!("/api/groups/{}/calendar", fixture.group.get());

    let (status, body) = call(
        &fixture.app,
        Method::GET,
        &path,
        Some(auth_header(&outsider_init)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "NOT_A_MEMBER");

    let no_tz_init = authenticate(&fixture.app, 42_102, "Без зоны").await;
    let (status, body) = call(
        &fixture.app,
        Method::GET,
        &path,
        Some(auth_header(&no_tz_init)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "TIMEZONE_REQUIRED");
}
