pub mod auth;
mod best_time;
mod calendar;
mod error;
mod groups;
mod health;
mod me;
mod preferences;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use axum::{
    Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use sqlx::SqlitePool;
use teloxide::Bot;
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::{auth::AuthSettings, services::clock::Clock};

pub use error::ApiError;

const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;

/// How long a rendered timetable may be reused for identical calendar input.
pub const PNG_CACHE_TTL: Duration = Duration::from_secs(60);
/// Upper bound on cached PNGs; each entry is a few hundred kilobytes.
pub const PNG_CACHE_CAPACITY: usize = 16;
/// Concurrent `resvg` renders. Each render allocates a multi-megabyte pixmap.
pub const PNG_MAX_CONCURRENT_RENDERS: usize = 2;

/// Shared application state. Handlers must never derive identity from
/// client-supplied user IDs.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub auth: AuthSettings,
    /// Used only for best-effort owner notifications; absent in API-only mode.
    pub bot: Option<Bot>,
    /// Single injectable clock for deterministic reference-week calculations.
    pub clock: Arc<dyn Clock>,
    /// Bounded PNG render cache and render concurrency limit.
    pub png: Arc<PngCache>,
}

#[derive(Debug)]
struct CachedPng {
    bytes: Bytes,
    created: Instant,
}

/// Short-lived, bounded cache for `…/timetable.png` plus a render semaphore.
///
/// The key is derived from the converted calendar content, so a changed
/// preference, timezone or reference week produces a new key.
#[derive(Debug)]
pub struct PngCache {
    entries: Mutex<HashMap<[u8; 32], CachedPng>>,
    permits: Semaphore,
}

impl PngCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            permits: Semaphore::new(PNG_MAX_CONCURRENT_RENDERS),
        }
    }

    pub fn get(&self, key: &[u8; 32]) -> Option<Bytes> {
        let mut entries = self.lock();
        let entry = entries.get(key)?;
        if entry.created.elapsed() > PNG_CACHE_TTL {
            entries.remove(key);
            return None;
        }
        Some(entry.bytes.clone())
    }

    pub fn put(&self, key: [u8; 32], bytes: Bytes) {
        let mut entries = self.lock();
        if entries.len() >= PNG_CACHE_CAPACITY && !entries.contains_key(&key) {
            entries.retain(|_, entry| entry.created.elapsed() <= PNG_CACHE_TTL);
            if entries.len() >= PNG_CACHE_CAPACITY {
                let oldest = entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.created)
                    .map(|(key, _)| *key);
                if let Some(oldest) = oldest {
                    entries.remove(&oldest);
                }
            }
        }
        entries.insert(
            key,
            CachedPng {
                bytes,
                created: Instant::now(),
            },
        );
    }

    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, ApiError> {
        self.permits
            .acquire()
            .await
            .map_err(|_| ApiError::internal())
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<[u8; 32], CachedPng>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for PngCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/auth/telegram", post(auth::telegram))
        .route("/api/me", get(me::get_me))
        .route("/api/me/timezone", put(me::update_timezone))
        .route("/api/groups", get(groups::list_groups))
        .route(
            "/api/groups/{group_id}/preferences",
            get(preferences::get_preferences).put(preferences::update_preferences),
        )
        .route(
            "/api/groups/{group_id}/calendar",
            get(calendar::get_group_calendar),
        )
        .route(
            "/api/groups/{group_id}/members/{member_id}/calendar",
            get(calendar::get_member_calendar),
        )
        .route(
            "/api/groups/{group_id}/best-time",
            get(best_time::get_best_time),
        )
        .route(
            "/api/groups/{group_id}/timetable.png",
            get(best_time::get_timetable_png),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7; 32];

    #[test]
    fn caches_and_returns_identical_bytes() {
        let cache = PngCache::new();
        assert!(cache.get(&KEY).is_none());

        cache.put(KEY, Bytes::from_static(b"png"));
        assert_eq!(cache.get(&KEY).as_deref(), Some(&b"png"[..]));
    }

    #[test]
    fn evicts_the_oldest_entry_at_capacity() {
        let cache = PngCache::new();
        for index in 0..PNG_CACHE_CAPACITY {
            cache.put([index as u8; 32], Bytes::from(vec![index as u8]));
        }
        assert!(
            cache.get(&[0; 32]).is_some(),
            "the first entry is still cached"
        );

        cache.put([200; 32], Bytes::from_static(b"new"));
        assert_eq!(cache.get(&[200; 32]).as_deref(), Some(&b"new"[..]));
        assert!(
            cache.lock().len() <= PNG_CACHE_CAPACITY,
            "cache must stay within its capacity"
        );
    }
}
