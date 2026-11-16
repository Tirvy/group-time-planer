use std::collections::HashSet;

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use url::form_urlencoded;

use crate::domain::{IdError, TelegramId};

type HmacSha256 = Hmac<Sha256>;

/// Tolerated clock skew for `auth_date` values slightly in the future.
pub const FUTURE_SKEW_SECS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramIdentity {
    pub telegram_id: TelegramId,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InitDataError {
    #[error("initData is missing")]
    Missing,
    #[error("initData is malformed")]
    Malformed,
    #[error("initData signature is invalid")]
    InvalidHash,
    #[error("initData has expired")]
    Expired,
    #[error("initData timestamp is too far in the future")]
    FutureTimestamp,
    #[error("initData does not contain a user")]
    MissingUser,
    #[error("initData contains an invalid user id")]
    InvalidUserId,
}

/// Validates Telegram Mini App `initData` using the documented HMAC scheme:
///
/// ```text
/// secret_key = HMAC_SHA256(key = "WebAppData", message = bot_token)
/// hash       = HMAC_SHA256(key = secret_key, message = data_check_string)
/// ```
///
/// `data_check_string` contains every received field except `hash`, sorted by
/// key, as `key=value` joined by `\n`. Values are URL-decoded before hashing.
pub fn validate(
    init_data: &str,
    bot_token: &str,
    now_secs: i64,
    max_age_secs: i64,
) -> Result<TelegramIdentity, InitDataError> {
    if init_data.trim().is_empty() {
        return Err(InitDataError::Missing);
    }

    let pairs: Vec<(String, String)> = form_urlencoded::parse(init_data.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    let mut seen = HashSet::with_capacity(pairs.len());
    for (key, _) in &pairs {
        if !seen.insert(key.as_str()) {
            return Err(InitDataError::Malformed);
        }
    }

    let hash = find(&pairs, "hash").ok_or(InitDataError::Malformed)?;
    if hash.len() != 64 {
        return Err(InitDataError::InvalidHash);
    }

    let auth_date_raw = find(&pairs, "auth_date").ok_or(InitDataError::Malformed)?;
    let auth_date = auth_date_raw
        .parse::<i64>()
        .map_err(|_| InitDataError::Malformed)?;

    let mut fields: Vec<&(String, String)> =
        pairs.iter().filter(|(key, _)| key != "hash").collect();
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    let data_check_string = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut key_mac =
        HmacSha256::new_from_slice(b"WebAppData").map_err(|_| InitDataError::Malformed)?;
    key_mac.update(bot_token.as_bytes());
    let secret_key = key_mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&secret_key).map_err(|_| InitDataError::Malformed)?;
    mac.update(data_check_string.as_bytes());

    let expected_hash = hex::decode(hash).map_err(|_| InitDataError::InvalidHash)?;
    mac.verify_slice(&expected_hash)
        .map_err(|_| InitDataError::InvalidHash)?;

    if auth_date > now_secs.saturating_add(FUTURE_SKEW_SECS) {
        return Err(InitDataError::FutureTimestamp);
    }
    if now_secs.saturating_sub(auth_date) > max_age_secs {
        return Err(InitDataError::Expired);
    }

    let user_json = find(&pairs, "user").ok_or(InitDataError::MissingUser)?;
    let user: InitDataUser =
        serde_json::from_str(user_json).map_err(|_| InitDataError::Malformed)?;

    let raw_id = i64::try_from(user.id).map_err(|_| InitDataError::InvalidUserId)?;
    let telegram_id = TelegramId::new(raw_id).map_err(|error: IdError| {
        let _ = error;
        InitDataError::InvalidUserId
    })?;

    Ok(TelegramIdentity {
        telegram_id,
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
    })
}

fn find<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(field, _)| field == key)
        .map(|(_, value)| value.as_str())
}

#[derive(Debug, Deserialize)]
struct InitDataUser {
    id: u64,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "123456:TEST-TOKEN";

    fn user_json(id: u64, first: &str) -> String {
        format!(r#"{{"id":{id},"first_name":"{first}","last_name":"Тестов","username":"tester"}}"#)
    }

    fn sign(token: &str, fields: &[(&str, String)]) -> String {
        let mut sorted: Vec<(&str, &str)> = fields
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        sorted.sort_by(|left, right| left.0.cmp(right.0));
        let data_check_string = sorted
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut key_mac = HmacSha256::new_from_slice(b"WebAppData").unwrap();
        key_mac.update(token.as_bytes());
        let secret_key = key_mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&secret_key).unwrap();
        mac.update(data_check_string.as_bytes());
        let hash = hex::encode(mac.finalize().into_bytes());

        let mut serializer = form_urlencoded::Serializer::new(String::new());
        for (key, value) in fields {
            serializer.append_pair(key, value);
        }
        serializer.append_pair("hash", &hash);
        serializer.finish()
    }

    fn init_data(token: &str, id: u64, auth_date: i64) -> String {
        sign(
            token,
            &[
                ("auth_date", auth_date.to_string()),
                ("query_id", "AAH-test".to_owned()),
                ("user", user_json(id, "Иван")),
            ],
        )
    }

    #[test]
    fn accepts_valid_init_data_and_parses_identity() {
        let now = 1_700_000_000;
        let data = init_data(TOKEN, 42, now);
        let identity = validate(&data, TOKEN, now, 3600).unwrap();

        assert_eq!(identity.telegram_id.get(), 42);
        assert_eq!(identity.first_name.as_deref(), Some("Иван"));
        assert_eq!(identity.last_name.as_deref(), Some("Тестов"));
        assert_eq!(identity.username.as_deref(), Some("tester"));
    }

    #[test]
    fn accepts_optional_signature_field() {
        let now = 1_700_000_000;
        let data = sign(
            TOKEN,
            &[
                ("auth_date", now.to_string()),
                ("signature", "ed25519-signature".to_owned()),
                ("user", user_json(7, "Анна")),
            ],
        );
        assert!(validate(&data, TOKEN, now, 3600).is_ok());
    }

    #[test]
    fn rejects_a_different_bot_token() {
        let now = 1_700_000_000;
        let data = init_data(TOKEN, 42, now);
        assert_eq!(
            validate(&data, "999:OTHER", now, 3600),
            Err(InitDataError::InvalidHash)
        );
    }

    #[test]
    fn rejects_tampered_fields() {
        let now = 1_700_000_000;
        let data = init_data(TOKEN, 42, now);
        let tampered = data.replace("first_name", "first_namf");
        assert_eq!(
            validate(&tampered, TOKEN, now, 3600),
            Err(InitDataError::InvalidHash)
        );
    }

    #[test]
    fn rejects_expired_and_future_timestamps() {
        let now = 1_700_000_000;

        let expired = init_data(TOKEN, 42, now - 3601);
        assert_eq!(
            validate(&expired, TOKEN, now, 3600),
            Err(InitDataError::Expired)
        );

        let future = init_data(TOKEN, 42, now + FUTURE_SKEW_SECS + 1);
        assert_eq!(
            validate(&future, TOKEN, now, 3600),
            Err(InitDataError::FutureTimestamp)
        );

        let slightly_future = init_data(TOKEN, 42, now + FUTURE_SKEW_SECS);
        assert!(validate(&slightly_future, TOKEN, now, 3600).is_ok());
    }

    #[test]
    fn rejects_missing_or_duplicate_fields() {
        assert_eq!(validate("", TOKEN, 0, 3600), Err(InitDataError::Missing));

        let no_hash = "auth_date=1700000000&user=%7B%22id%22%3A1%7D";
        assert_eq!(
            validate(no_hash, TOKEN, 1_700_000_000, 3600),
            Err(InitDataError::Malformed)
        );

        let duplicate = format!(
            "{}&auth_date=1700000000",
            init_data(TOKEN, 42, 1_700_000_000)
        );
        assert_eq!(
            validate(&duplicate, TOKEN, 1_700_000_000, 3600),
            Err(InitDataError::Malformed)
        );
    }

    #[test]
    fn rejects_missing_user_and_invalid_hash_encoding() {
        let now = 1_700_000_000;
        let no_user = sign(TOKEN, &[("auth_date", now.to_string())]);
        assert_eq!(
            validate(&no_user, TOKEN, now, 3600),
            Err(InitDataError::MissingUser)
        );

        let data = init_data(TOKEN, 42, now);
        let hash_start = data.find("hash=").unwrap();
        let invalid_hash = format!("{}hash=zz", &data[..hash_start]);
        assert_eq!(
            validate(&invalid_hash, TOKEN, now, 3600),
            Err(InitDataError::InvalidHash)
        );
    }

    #[test]
    fn rejects_ids_outside_signed_64_bit_range() {
        let now = 1_700_000_000;
        let data = init_data(TOKEN, i64::MAX as u64 + 1, now);
        assert_eq!(
            validate(&data, TOKEN, now, 3600),
            Err(InitDataError::InvalidUserId)
        );
    }
}
