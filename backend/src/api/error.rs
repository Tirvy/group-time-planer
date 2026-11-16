use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::auth::{AuthUserError, InitDataError};

/// Stable API error envelope. Frontend maps `code` to Russian copy.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

impl ApiError {
    pub const fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
        }
    }

    pub fn auth_missing() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_MISSING",
            "Telegram authentication is required",
        )
    }

    pub fn auth_unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AUTH_UNAVAILABLE",
            "Telegram authentication is not configured",
        )
    }

    pub fn validation(message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message)
    }

    pub fn timezone_invalid() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "TIMEZONE_INVALID",
            "Unknown IANA timezone",
        )
    }

    pub fn timezone_required() -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "TIMEZONE_REQUIRED",
            "The user must select a timezone first",
        )
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Internal server error",
        )
    }

    pub fn from_auth(error: AuthUserError) -> Self {
        match error {
            AuthUserError::Unavailable => Self::auth_unavailable(),
            AuthUserError::InitData(InitDataError::Expired) => Self::new(
                StatusCode::UNAUTHORIZED,
                "AUTH_EXPIRED",
                "Telegram initData has expired",
            ),
            AuthUserError::InitData(_) => Self::new(
                StatusCode::UNAUTHORIZED,
                "AUTH_INVALID",
                "Telegram initData is invalid",
            ),
            AuthUserError::User(_) => {
                tracing::error!("authentication failed: storage error");
                Self::internal()
            }
        }
    }
}
