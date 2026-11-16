use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

/// Runtime configuration loaded from environment variables (optionally via `.env`).
///
/// Secrets are intentionally not part of this struct yet: the Telegram bot token is
/// introduced in Phase 2 together with a redacting wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_path: PathBuf,
    pub db_max_connections: u32,
    pub frontend_dist: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("APP_BIND_ADDR is not a valid host:port socket address: {value:?}")]
    InvalidBindAddr { value: String },
    #[error("DB_MAX_CONNECTIONS must be an integer between 1 and 16, got {value:?}")]
    InvalidMaxConnections { value: String },
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
        }
    }
}

impl Config {
    pub const DEFAULT_BIND_ADDR: &'static str = "127.0.0.1:8080";
    pub const DEFAULT_DATABASE_PATH: &'static str = "data/app.db";
    pub const DEFAULT_FRONTEND_DIST: &'static str = "frontend/dist";
    pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;
    pub const MAX_CONNECTIONS_LIMIT: u32 = 16;

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

        Ok(Self {
            bind_addr,
            database_path,
            db_max_connections,
            frontend_dist,
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
    }

    #[test]
    fn trims_and_parses_overrides() {
        let config = config_from(&[
            ("APP_BIND_ADDR", " 0.0.0.0:9000 "),
            ("DATABASE_PATH", " data/test.db "),
            ("DB_MAX_CONNECTIONS", "2"),
            ("FRONTEND_DIST", "web"),
        ])
        .expect("overrides are valid");
        assert_eq!(config.bind_addr, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(config.database_path, PathBuf::from("data/test.db"));
        assert_eq!(config.db_max_connections, 2);
        assert_eq!(config.frontend_dist, PathBuf::from("web"));
    }

    #[test]
    fn rejects_invalid_bind_addr() {
        let error = config_from(&[("APP_BIND_ADDR", "not-a-socket")]).unwrap_err();
        assert!(matches!(error, ConfigError::InvalidBindAddr { .. }));
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
