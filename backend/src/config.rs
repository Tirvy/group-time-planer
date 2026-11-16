use std::{
    fmt,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

/// Runtime configuration loaded from environment variables (optionally via `.env`).
///
/// Secrets are wrapped so `Debug` output and accidental formatting cannot leak
/// them into logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_path: PathBuf,
    pub db_max_connections: u32,
    pub frontend_dist: PathBuf,
    pub telegram_bot_token: Option<SecretString>,
    /// Maximum accepted age of Telegram Mini App `initData`.
    pub init_data_max_age: Duration,
    /// HTTPS origin used for Telegram Web App buttons. No query or fragment.
    pub public_base_url: Option<String>,
}

/// A secret that never prints its value through `Debug` or `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Explicit access for constructing API clients. Never log the result.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("APP_BIND_ADDR is not a valid host:port socket address: {value:?}")]
    InvalidBindAddr { value: String },
    #[error("DB_MAX_CONNECTIONS must be an integer between 1 and 16, got {value:?}")]
    InvalidMaxConnections { value: String },
    #[error("INIT_DATA_MAX_AGE_SECS must be between 60 and 604800, got {value:?}")]
    InvalidInitDataMaxAge { value: String },
    #[error("PUBLIC_BASE_URL must be an https URL without query or fragment, got {value:?}")]
    InvalidPublicBaseUrl { value: String },
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: Self::DEFAULT_BIND_ADDR
                .parse()
                .expect("default bind address is valid"),
            database_path: PathBuf::from(Self::DEFAULT_DATABASE_PATH),
            db_max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            frontend_dist: PathBuf::from(Self::DEFAULT_FRONTEND_DIST),
            telegram_bot_token: None,
            init_data_max_age: Duration::from_secs(Self::DEFAULT_INIT_DATA_MAX_AGE_SECS),
            public_base_url: None,
        }
    }
}

impl Config {
    pub const DEFAULT_BIND_ADDR: &'static str = "127.0.0.1:8080";
    pub const DEFAULT_DATABASE_PATH: &'static str = "data/app.db";
    pub const DEFAULT_FRONTEND_DIST: &'static str = "frontend/dist";
    pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;
    pub const MAX_CONNECTIONS_LIMIT: u32 = 16;
    pub const DEFAULT_INIT_DATA_MAX_AGE_SECS: u64 = 86_400;
    pub const MIN_INIT_DATA_MAX_AGE_SECS: u64 = 60;
    pub const MAX_INIT_DATA_MAX_AGE_SECS: u64 = 604_800;

    /// Loads configuration from the process environment.
    pub fn load() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Builds configuration from an injected lookup, which keeps parsing testable
    /// without mutating global process state.
    pub fn from_lookup<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let bind_addr_raw =
            value(&lookup, "APP_BIND_ADDR").unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned());
        let bind_addr =
            bind_addr_raw
                .parse::<SocketAddr>()
                .map_err(|_| ConfigError::InvalidBindAddr {
                    value: bind_addr_raw.clone(),
                })?;

        let database_path = match value(&lookup, "DATABASE_PATH") {
            Some(raw) => PathBuf::from(raw),
            None => PathBuf::from(Self::DEFAULT_DATABASE_PATH),
        };

        let db_max_connections = match value(&lookup, "DB_MAX_CONNECTIONS") {
            Some(raw) => {
                let parsed = raw
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=Self::MAX_CONNECTIONS_LIMIT).contains(n));
                parsed.ok_or(ConfigError::InvalidMaxConnections { value: raw })?
            }
            None => Self::DEFAULT_MAX_CONNECTIONS,
        };

        let frontend_dist = match value(&lookup, "FRONTEND_DIST") {
            Some(raw) => PathBuf::from(raw),
            None => PathBuf::from(Self::DEFAULT_FRONTEND_DIST),
        };

        let telegram_bot_token = value(&lookup, "TELOXIDE_TOKEN").map(SecretString::new);

        let init_data_max_age = match value(&lookup, "INIT_DATA_MAX_AGE_SECS") {
            Some(raw) => {
                let secs = raw
                    .parse::<u64>()
                    .ok()
                    .filter(|seconds| {
                        (Self::MIN_INIT_DATA_MAX_AGE_SECS..=Self::MAX_INIT_DATA_MAX_AGE_SECS)
                            .contains(seconds)
                    })
                    .ok_or(ConfigError::InvalidInitDataMaxAge { value: raw })?;
                Duration::from_secs(secs)
            }
            None => Duration::from_secs(Self::DEFAULT_INIT_DATA_MAX_AGE_SECS),
        };

        let public_base_url = match value(&lookup, "PUBLIC_BASE_URL") {
            Some(raw) => {
                let parsed = url::Url::parse(&raw).ok().filter(|url| {
                    url.scheme() == "https"
                        && url.host().is_some()
                        && url.query().is_none()
                        && url.fragment().is_none()
                });
                Some(
                    parsed
                        .ok_or(ConfigError::InvalidPublicBaseUrl { value: raw })?
                        .to_string(),
                )
            }
            None => None,
        };

        Ok(Self {
            bind_addr,
            database_path,
            db_max_connections,
            frontend_dist,
            telegram_bot_token,
            init_data_max_age,
            public_base_url,
        })
    }

    pub fn database_dir(&self) -> Option<&Path> {
        self.database_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
    }
}

fn value<F>(lookup: &F, key: &str) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(key)
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_from(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|key| map.get(key).cloned())
    }

    #[test]
    fn applies_defaults_when_environment_is_empty() {
        let config = config_from(&[]).expect("defaults are valid");
        assert_eq!(config.bind_addr.to_string(), Config::DEFAULT_BIND_ADDR);
        assert_eq!(
            config.database_path,
            PathBuf::from(Config::DEFAULT_DATABASE_PATH)
        );
        assert_eq!(config.db_max_connections, Config::DEFAULT_MAX_CONNECTIONS);
        assert_eq!(
            config.frontend_dist,
            PathBuf::from(Config::DEFAULT_FRONTEND_DIST)
        );
        assert!(config.telegram_bot_token.is_none());
        assert_eq!(
            config.init_data_max_age,
            Duration::from_secs(Config::DEFAULT_INIT_DATA_MAX_AGE_SECS)
        );
    }

    #[test]
    fn loads_bot_token_without_exposing_it() {
        let secret = "123456789:AAH-secret-token-value";
        let config = config_from(&[("TELOXIDE_TOKEN", secret)]).expect("token is valid");

        let token = config.telegram_bot_token.as_ref().expect("token loaded");
        assert_eq!(token.expose(), secret);
        assert_eq!(token.to_string(), "[REDACTED]");
        assert!(!format!("{config:?}").contains(secret));
        assert!(!format!("{token:?}").contains(secret));
    }

    #[test]
    fn blank_bot_token_is_treated_as_unset() {
        let config = config_from(&[("TELOXIDE_TOKEN", "   ")]).unwrap();
        assert!(config.telegram_bot_token.is_none());
    }

    #[test]
    fn trims_and_parses_overrides() {
        let config = config_from(&[
            ("APP_BIND_ADDR", " 0.0.0.0:9000 "),
            ("DATABASE_PATH", " data/test.db "),
            ("DB_MAX_CONNECTIONS", "2"),
            ("FRONTEND_DIST", "web"),
            ("INIT_DATA_MAX_AGE_SECS", "3600"),
        ])
        .expect("overrides are valid");
        assert_eq!(config.bind_addr, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(config.database_path, PathBuf::from("data/test.db"));
        assert_eq!(config.db_max_connections, 2);
        assert_eq!(config.frontend_dist, PathBuf::from("web"));
        assert_eq!(config.init_data_max_age, Duration::from_secs(3600));
    }

    #[test]
    fn rejects_out_of_range_init_data_age() {
        for value in ["0", "59", "604801", "abc", "-1"] {
            let error = config_from(&[("INIT_DATA_MAX_AGE_SECS", value)]).unwrap_err();
            assert!(
                matches!(error, ConfigError::InvalidInitDataMaxAge { .. }),
                "expected rejection for {value}"
            );
        }
    }

    #[test]
    fn rejects_invalid_bind_addr() {
        let error = config_from(&[("APP_BIND_ADDR", "not-a-socket")]).unwrap_err();
        assert!(matches!(error, ConfigError::InvalidBindAddr { .. }));
    }

    #[test]
    fn loads_and_normalizes_public_base_url() {
        let config = config_from(&[("PUBLIC_BASE_URL", "https://scheduler.example.com")]).unwrap();
        assert_eq!(
            config.public_base_url.as_deref(),
            Some("https://scheduler.example.com/")
        );

        let blank = config_from(&[("PUBLIC_BASE_URL", "   ")]).unwrap();
        assert!(blank.public_base_url.is_none());
    }

    #[test]
    fn rejects_invalid_public_base_urls() {
        for value in [
            "http://example.com",
            "ftp://example.com",
            "not-a-url",
            "https://example.com?x=1",
            "https://example.com/#frag",
        ] {
            let error = config_from(&[("PUBLIC_BASE_URL", value)]).unwrap_err();
            assert!(
                matches!(error, ConfigError::InvalidPublicBaseUrl { .. }),
                "expected rejection for {value}"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_connection_counts() {
        for value in ["0", "17", "abc", "-1"] {
            let error = config_from(&[("DB_MAX_CONNECTIONS", value)]).unwrap_err();
            assert!(
                matches!(error, ConfigError::InvalidMaxConnections { .. }),
                "expected rejection for {value}"
            );
        }
    }

    #[test]
    fn blank_values_fall_back_to_defaults() {
        let config = config_from(&[
            ("APP_BIND_ADDR", "   "),
            ("DATABASE_PATH", ""),
            ("DB_MAX_CONNECTIONS", "  "),
            ("FRONTEND_DIST", "\t"),
        ])
        .expect("blank values are treated as unset");
        assert_eq!(config.bind_addr.to_string(), Config::DEFAULT_BIND_ADDR);
        assert_eq!(
            config.database_path,
            PathBuf::from(Config::DEFAULT_DATABASE_PATH)
        );
        assert_eq!(config.db_max_connections, Config::DEFAULT_MAX_CONNECTIONS);
        assert_eq!(
            config.frontend_dist,
            PathBuf::from(Config::DEFAULT_FRONTEND_DIST)
        );
    }

    #[test]
    fn blank_database_path_uses_default() {
        let config = config_from(&[("DATABASE_PATH", "   ")]).unwrap();
        assert_eq!(
            config.database_path,
            PathBuf::from(Config::DEFAULT_DATABASE_PATH)
        );
        assert_eq!(config.database_dir(), Some(Path::new("data")));
    }
}
