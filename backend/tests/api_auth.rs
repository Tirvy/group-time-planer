mod common;

use std::time::Duration;

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
use raid_scheduler_backend::{auth::AuthSettings, domain::TelegramId};
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

#[tokio::test]
async fn auth_endpoint_creates_user_and_reports_timezone_state() {
    let (app, db) = test_api().await;
    let init_data = sign_init_data(TEST_BOT_TOKEN, 700_001, now_secs(), "Иван");

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["telegram_id"], "700001");
    assert_eq!(body["user"]["first_name"], "Иван");
    assert_eq!(body["user"]["timezone_set"], false);
    assert!(body["user"]["timezone"].is_null());
    assert_eq!(count(&db.pool, "SELECT COUNT(*) FROM users").await, 1);
}

#[tokio::test]
async fn auth_rejects_tampered_and_expired_init_data() {
    let (app, db) = test_api().await;

    let wrong_token = sign_init_data("999:OTHER-TOKEN", 700_002, now_secs(), "Пётр");
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": wrong_token })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "AUTH_INVALID");

    let expired = sign_init_data(TEST_BOT_TOKEN, 700_003, now_secs() - 3601, "Пётр");
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": expired })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "AUTH_EXPIRED");

    assert_eq!(
        count(&db.pool, "SELECT COUNT(*) FROM users").await,
        0,
        "invalid authentication must not create users"
    );
}

#[tokio::test]
async fn protected_routes_require_a_valid_tma_header() {
    let (app, _db) = test_api().await;
    let init_data = sign_init_data(TEST_BOT_TOKEN, 700_101, now_secs(), "Анна");

    let (status, body) = call(&app, Method::GET, "/api/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "AUTH_MISSING");

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/me",
        Some(format!("Bearer {init_data}")),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "AUTH_MISSING");

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/me",
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["telegram_id"], "700101");
}

#[tokio::test]
async fn timezone_update_uses_server_identity_and_ignores_client_user_ids() {
    let (app, db) = test_api().await;
    let init_a = sign_init_data(TEST_BOT_TOKEN, 700_201, now_secs(), "Алиса");
    let init_b = sign_init_data(TEST_BOT_TOKEN, 700_202, now_secs(), "Борис");

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_b })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        &app,
        Method::PUT,
        "/api/me/timezone",
        Some(auth_header(&init_a)),
        Some(json!({
            "timezone": "Asia/Tbilisi",
            "user_id": 700_202,
            "telegram_id": "700202"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["telegram_id"], "700201");
    assert_eq!(body["user"]["timezone"], "Asia/Tbilisi");
    assert_eq!(body["user"]["timezone_set"], true);

    let timezone_a: Option<String> =
        sqlx::query_scalar("SELECT timezone FROM users WHERE telegram_id = 700201")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(timezone_a.as_deref(), Some("Asia/Tbilisi"));

    let timezone_b: Option<String> =
        sqlx::query_scalar("SELECT timezone FROM users WHERE telegram_id = 700202")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(timezone_b, None, "client-supplied IDs must be ignored");

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/me",
        Some(auth_header(&init_b)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["timezone_set"], false);
}

#[tokio::test]
async fn timezone_update_validates_iana_identifiers_and_persists() {
    let (app, db) = test_api().await;
    let init_data = sign_init_data(TEST_BOT_TOKEN, 700_301, now_secs(), "Мария");

    let (status, body) = call(
        &app,
        Method::PUT,
        "/api/me/timezone",
        Some(auth_header(&init_data)),
        Some(json!({ "timezone": "Mars/Olympus" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "TIMEZONE_INVALID");

    let (status, body) = call(
        &app,
        Method::PUT,
        "/api/me/timezone",
        Some(auth_header(&init_data)),
        Some(json!({ "timezone": "Europe/Berlin" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["timezone"], "Europe/Berlin");
    assert_eq!(body["user"]["timezone_set"], true);

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/me",
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["timezone"], "Europe/Berlin");
    assert_eq!(body["user"]["timezone_set"], true);

    let stored: String =
        sqlx::query_scalar("SELECT timezone FROM users WHERE telegram_id = 700301")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(stored, "Europe/Berlin");
}

#[tokio::test]
async fn auth_is_unavailable_without_a_configured_bot_token() {
    let test_db = test_db().await;
    let app = test_router(
        &test_db.pool,
        AuthSettings {
            bot_token: None,
            max_age: Duration::from_secs(3600),
        },
    );
    let init_data = sign_init_data(TEST_BOT_TOKEN, 700_401, now_secs(), "Олег");

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "AUTH_UNAVAILABLE");

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/me",
        Some(auth_header(&init_data)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "AUTH_UNAVAILABLE");
}

#[tokio::test]
async fn telegram_identifiers_are_serialized_as_strings() {
    let (app, db) = test_api().await;
    let raw_id = 700_501;
    let init_data = sign_init_data(TEST_BOT_TOKEN, raw_id, now_secs(), "Строка");

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/auth/telegram",
        None,
        Some(json!({ "initData": init_data })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["user"]["telegram_id"].is_string());

    let stored: i64 = sqlx::query_scalar("SELECT telegram_id FROM users WHERE id = 1")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(TelegramId::new(stored).unwrap().get(), raw_id);
}
