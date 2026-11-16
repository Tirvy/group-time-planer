//! Opaque invitation tokens. Raw tokens are never stored in the database;
//! only their SHA-256 hash is persisted.

use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// 256 bits of entropy by default.
pub const TOKEN_BYTES: usize = 32;
/// Base64url without padding encodes 32 bytes into 43 characters.
pub const TOKEN_CHARS: usize = 43;

/// Raw URL-safe invitation token. Debug output is redacted so tokens cannot
/// leak into logs accidentally.
#[derive(Clone, PartialEq, Eq)]
pub struct InvitationToken(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvitationTokenError {
    #[error("invalid invitation token format")]
    Invalid,
}

impl InvitationToken {
    /// Generates a cryptographically random token from the OS-backed CSPRNG.
    pub fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_BYTES];
        rand::rng().fill_bytes(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Validates a token received from a deep link or callback before hashing.
    pub fn from_raw(raw: &str) -> Result<Self, InvitationTokenError> {
        if raw.len() != TOKEN_CHARS
            || !raw
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(InvitationTokenError::Invalid);
        }

        let decoded = URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(|_| InvitationTokenError::Invalid)?;
        if decoded.len() != TOKEN_BYTES {
            return Err(InvitationTokenError::Invalid);
        }

        Ok(Self(raw.to_owned()))
    }

    /// SHA-256 hash as lowercase hex (64 characters).
    pub fn hash(&self) -> String {
        hex::encode(Sha256::digest(self.0.as_bytes()))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Telegram deep-link start payload: `join_<token>`.
    pub fn start_payload(&self) -> String {
        format!("join_{}", self.0)
    }

    /// Extracts a token from a `join_<token>` start payload.
    pub fn from_start_payload(payload: &str) -> Result<Self, InvitationTokenError> {
        let raw = payload
            .strip_prefix("join_")
            .ok_or(InvitationTokenError::Invalid)?;
        Self::from_raw(raw)
    }
}

impl fmt::Debug for InvitationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InvitationToken([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_have_expected_entropy_shape() {
        let token = InvitationToken::generate();
        assert_eq!(token.expose().len(), TOKEN_CHARS);
        assert!(
            token
                .expose()
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
        assert_ne!(token.expose(), InvitationToken::generate().expose());
    }

    #[test]
    fn hash_is_sha256_hex_and_deterministic() {
        let raw = "A".repeat(TOKEN_CHARS);
        let token = InvitationToken::from_raw(&raw).unwrap();
        let hash = token.hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(hash, token.hash());
    }

    #[test]
    fn round_trips_start_payloads() {
        let token = InvitationToken::generate();
        let payload = token.start_payload();
        assert!(
            payload.len() <= 64,
            "Telegram start payload must fit in 64 characters"
        );
        let parsed = InvitationToken::from_start_payload(&payload).unwrap();
        assert_eq!(parsed.expose(), token.expose());
    }

    #[test]
    fn rejects_malformed_tokens() {
        for raw in [
            String::new(),
            "short".to_owned(),
            "../".repeat(20),
            "!".repeat(TOKEN_CHARS),
            "A".repeat(TOKEN_CHARS + 1),
            "+".repeat(TOKEN_CHARS),
        ] {
            assert!(
                InvitationToken::from_raw(&raw).is_err(),
                "{raw:?} must be rejected"
            );
        }
        assert!(InvitationToken::from_start_payload("start_abc").is_err());
        assert!(InvitationToken::from_start_payload(&format!("join_{}", "A".repeat(10))).is_err());
    }

    #[test]
    fn debug_output_is_redacted() {
        let token = InvitationToken::generate();
        let raw = token.expose().to_owned();
        assert!(!format!("{token:?}").contains(&raw));
    }
}
