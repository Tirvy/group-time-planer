mod common;

use axum::{
    Router,
    body::Body,
    http::{
        Method, Request, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use common::*;
use http_body_util::BodyExt;
use raid_scheduler_backend::domain::{MEMBER_COLORS, UserId};
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

async fn set_timezone(app: &Router, init_data: &str, timezone: &str) {
    let (status, body) = call(
        app,
        Method::PUT,
        "/api/me/timezone",
        Some(auth_header(init_data)),
        Some(json!({ "timezone": timezone })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "timezone setup failed: {body}");
}

#[tokio::test]
async fn groups_and_preferences_round_trip_with_empty_save_completion() {
    let (app, db) = test_api().await;
    let telegram_id = 41_001;
    let init_data = sign_init_data(TEST_BOT_TOKEN, telegram_id, now_secs(), "Владелец");

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let owner = internal_user_id(&db.pool, telegram_id).await;
    let group = seed_group(&db.pool, owner, "Календарь").await;
    set_timezone(&app, &init_data, "Asia/Tbilisi").await;
    let path = format!("/api/groups/{}/preferences", group.id.get());

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/groups",
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["groups"].as_array().unwrap().len(), 1);
    assert_eq!(body["groups"][0]["id"], group.id.get().to_string());
    assert_eq!(body["groups"][0]["role"], "owner");
    assert_eq!(body["groups"][0]["color"], MEMBER_COLORS[0]);
    assert_eq!(body["groups"][0]["setup_completed"], false);

    let (status, body) = call(
        &app,
        Method::GET,
        &path,
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["timezone"], "Asia/Tbilisi");
    assert_eq!(body["setup_completed"], false);
    assert_eq!(body["intervals"].as_array().unwrap().len(), 0);

    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(auth_header(&init_data)),
        Some(json!({
            "intervals": [
                { "weekday": 3, "start_minute": 660, "end_minute": 720 },
                { "weekday": 1, "start_minute": 600, "end_minute": 660 }
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["setup_completed"], true);
    assert_eq!(body["intervals"][0]["weekday"], 1);
    assert_eq!(body["intervals"][1]["weekday"], 3);

    let (status, body) = call(
        &app,
        Method::GET,
        &path,
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["setup_completed"], true);
    assert_eq!(body["intervals"].as_array().unwrap().len(), 2);

    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(auth_header(&init_data)),
        Some(json!({ "intervals": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["setup_completed"], true, "empty save stays completed");
    assert_eq!(body["intervals"].as_array().unwrap().len(), 0);
    assert_eq!(
        count(&db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        0
    );

    let (_, body) = call(
        &app,
        Method::GET,
        "/api/groups",
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(body["groups"][0]["setup_completed"], true);
}

#[tokio::test]
async fn preference_validation_returns_stable_errors_and_writes_nothing() {
    let (app, db) = test_api().await;
    let telegram_id = 41_101;
    let init_data = sign_init_data(TEST_BOT_TOKEN, telegram_id, now_secs(), "Валидатор");

    call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;
    let owner = internal_user_id(&db.pool, telegram_id).await;
    let group = seed_group(&db.pool, owner, "Ошибки").await;
    set_timezone(&app, &init_data, "Europe/Berlin").await;
    let path = format!("/api/groups/{}/preferences", group.id.get());

    let invalid_bodies = [
        json!({ "intervals": [
            { "weekday": 1, "start_minute": 600, "end_minute": 660 },
            { "weekday": 1, "start_minute": 630, "end_minute": 690 }
        ]}),
        json!({ "intervals": [{ "weekday": 0, "start_minute": 600, "end_minute": 660 }] }),
        json!({ "intervals": [{ "weekday": 8, "start_minute": 600, "end_minute": 660 }] }),
        json!({ "intervals": [{ "weekday": 1, "start_minute": 615, "end_minute": 660 }] }),
        json!({ "intervals": [{ "weekday": 1, "start_minute": 600, "end_minute": 600 }] }),
        json!({ "intervals": [{ "weekday": 1, "start_minute": 600, "end_minute": 1441 }] }),
    ];

    for body in invalid_bodies {
        let (status, response) = call(
            &app,
            Method::PUT,
            &path,
            Some(auth_header(&init_data)),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert_eq!(response["error"]["code"], "VALIDATION_ERROR");
    }

    assert_eq!(
        count(&db.pool, "SELECT COUNT(*) FROM weekly_preferences").await,
        0
    );
    let setup: Option<i64> = sqlx::query_scalar(
        "SELECT setup_completed_at FROM group_members WHERE group_id = ? AND user_id = ?",
    )
    .bind(group.id.get())
    .bind(owner.get())
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(setup.is_none());
}

#[tokio::test]
async fn preferences_require_membership_and_a_timezone() {
    let (app, db) = test_api().await;
    let owner_init = sign_init_data(TEST_BOT_TOKEN, 41_201, now_secs(), "Владелец");
    let outsider_init = sign_init_data(TEST_BOT_TOKEN, 41_202, now_secs(), "Чужой");
    let no_tz_init = sign_init_data(TEST_BOT_TOKEN, 41_203, now_secs(), "Без зоны");

    for init in [&owner_init, &outsider_init, &no_tz_init] {
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/auth/telegram",
            None,
            Some(json!({ "initData": init })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let owner = internal_user_id(&db.pool, 41_201).await;
    let group = seed_group(&db.pool, owner, "Закрытая").await;
    set_timezone(&app, &owner_init, "Asia/Tbilisi").await;
    set_timezone(&app, &outsider_init, "Europe/Berlin").await;

    let path = format!("/api/groups/{}/preferences", group.id.get());

    let (status, body) = call(
        &app,
        Method::GET,
        &path,
        Some(auth_header(&outsider_init)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "NOT_A_MEMBER");

    let (status, body) = call(
        &app,
        Method::PUT,
        &path,
        Some(auth_header(&outsider_init)),
        Some(json!({ "intervals": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "NOT_A_MEMBER");

    let (status, body) = call(
        &app,
        Method::GET,
        &path,
        Some(auth_header(&no_tz_init)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "TIMEZONE_REQUIRED");

    let decisions = [Method::GET, Method::PUT];
    for method in decisions {
        let body = if method == Method::PUT {
            Some(json!({ "intervals": [] }))
        } else {
            None
        };
        let (status, response) = call(
            &app,
            method,
            "/api/groups/999999/preferences",
            Some(auth_header(&owner_init)),
            body,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(response["error"]["code"], "NOT_A_MEMBER");
    }
}
