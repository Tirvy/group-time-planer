use std::time::Duration;

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::{api::AppState, db};

const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// `ok` when every dependency is healthy, otherwise `degraded`.
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    /// `ok` or `error`; never contains SQLite internals.
    pub database: &'static str,
}

/// Liveness/readiness endpoint. Returns 503 when the database cannot answer a
/// trivial query, so deploy tooling can distinguish "process up" from "usable".
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let database_ok = match tokio::time::timeout(HEALTH_TIMEOUT, db::ping(&state.db)).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "database health check failed");
            false
        }
        Err(_) => {
            tracing::warn!("database health check timed out");
            false
        }
    };

    let (status_code, status, database) = if database_ok {
        (StatusCode::OK, "ok", "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "degraded", "error")
    };

    (
        status_code,
        Json(HealthResponse {
            status,
            service: "raid-scheduler-backend",
            version: env!("CARGO_PKG_VERSION"),
            database,
        }),
    )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{api, config::Config, db};

    async fn test_app() -> (axum::Router, sqlx::SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_path: dir.path().join("app.db"),
            db_max_connections: 1,
            ..Config::default()
        };
        let pool = db::connect(&config).await.unwrap();
        (
            api::router(api::AppState {
                db: pool.clone(),
                auth: crate::auth::AuthSettings {
                    bot_token: None,
                    max_age: std::time::Duration::from_secs(60),
                },
                bot: None,
                clock: std::sync::Arc::new(crate::services::clock::SystemClock),
            }),
            pool,
            dir,
        )
    }

    async fn get_health(app: axum::Router) -> (StatusCode, serde_json::Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn health_reports_ok_and_version() {
        let (app, pool, _dir) = test_app().await;
        let (status, body) = get_health(app).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["database"], "ok");
        assert_eq!(body["service"], "raid-scheduler-backend");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));

        pool.close().await;
    }

    #[tokio::test]
    async fn health_reports_degraded_without_database() {
        let (app, pool, _dir) = test_app().await;
        pool.close().await;

        let (status, body) = get_health(app).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["database"], "error");
    }
}
