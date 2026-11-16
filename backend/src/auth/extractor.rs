use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::{
    api::{ApiError, AppState},
    auth,
    domain::IanaTimezone,
    services::users::User,
};

/// Authenticated Mini App user. The identity comes exclusively from validated
/// `initData`; client-supplied user IDs are never accepted.
pub struct AuthenticatedUser(pub User);

impl AuthenticatedUser {
    /// Calendar and schedule handlers must call this before returning data.
    pub fn require_timezone(&self) -> Result<IanaTimezone, ApiError> {
        self.0
            .timezone
            .as_deref()
            .ok_or_else(ApiError::timezone_required)
            .and_then(|raw| IanaTimezone::parse(raw).map_err(|_| ApiError::timezone_required()))
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(ApiError::auth_missing)?;

        let (scheme, init_data) = header.split_once(' ').ok_or_else(ApiError::auth_missing)?;
        if !scheme.eq_ignore_ascii_case("tma") {
            return Err(ApiError::auth_missing());
        }

        let init_data = init_data.trim();
        if init_data.is_empty() {
            return Err(ApiError::auth_missing());
        }

        let user = auth::authenticate(&state.db, &state.auth, init_data)
            .await
            .map_err(ApiError::from_auth)?;

        Ok(Self(user))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::domain::{TelegramId, UserId};

    fn user(timezone: Option<&str>) -> User {
        User {
            id: UserId::new(1).unwrap(),
            telegram_id: TelegramId::new(10).unwrap(),
            username: None,
            first_name: Some("Тест".to_owned()),
            last_name: None,
            timezone: timezone.map(str::to_owned),
        }
    }

    #[test]
    fn calendar_handlers_require_a_timezone() {
        let authenticated = AuthenticatedUser(user(None));
        let error = authenticated.require_timezone().unwrap_err();
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.code, "TIMEZONE_REQUIRED");

        let invalid = AuthenticatedUser(user(Some("Mars/Olympus")));
        assert_eq!(
            invalid.require_timezone().unwrap_err().code,
            "TIMEZONE_REQUIRED"
        );
    }

    #[test]
    fn returns_the_configured_iana_timezone() {
        let authenticated = AuthenticatedUser(user(Some("Asia/Tbilisi")));
        assert_eq!(
            authenticated.require_timezone().unwrap().name(),
            "Asia/Tbilisi"
        );
    }
}
