use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use crate::domain::{GroupId, UserId};

/// How long a user may take to send a group name after `/create`.
pub const CREATION_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeginCreation {
    Started,
    AlreadyPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationState {
    NotStarted,
    Awaiting,
    Expired,
}

#[derive(Debug, Default)]
struct Session {
    active_group: Option<GroupId>,
    creation_started_at: Option<Instant>,
}

/// In-memory per-user bot state for one process.
///
/// The MVP runs a single backend instance. Active group selection and pending
/// group creation are intentionally not persisted; after a restart the user is
/// asked to choose again instead of acting on stale context.
#[derive(Debug, Default)]
pub struct BotSessions {
    inner: Mutex<HashMap<UserId, Session>>,
}

impl BotSessions {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<UserId, Session>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn begin_creation(&self, user: UserId, now: Instant) -> BeginCreation {
        let mut sessions = self.lock();
        let session = sessions.entry(user).or_default();

        if let Some(started) = session.creation_started_at
            && now.saturating_duration_since(started) <= CREATION_TTL
        {
            return BeginCreation::AlreadyPending;
        }

        session.creation_started_at = Some(now);
        BeginCreation::Started
    }

    pub fn creation_state(&self, user: UserId, now: Instant) -> CreationState {
        let mut sessions = self.lock();
        let Some(session) = sessions.get_mut(&user) else {
            return CreationState::NotStarted;
        };

        match session.creation_started_at {
            None => CreationState::NotStarted,
            Some(started) if now.saturating_duration_since(started) > CREATION_TTL => {
                session.creation_started_at = None;
                CreationState::Expired
            }
            Some(_) => CreationState::Awaiting,
        }
    }

    pub fn clear_creation(&self, user: UserId) {
        if let Some(session) = self.lock().get_mut(&user) {
            session.creation_started_at = None;
        }
    }

    pub fn set_active_group(&self, user: UserId, group: GroupId) {
        self.lock().entry(user).or_default().active_group = Some(group);
    }

    pub fn clear_active_group(&self, user: UserId) {
        if let Some(session) = self.lock().get_mut(&user) {
            session.active_group = None;
        }
    }

    pub fn active_group(&self, user: UserId) -> Option<GroupId> {
        self.lock()
            .get(&user)
            .and_then(|session| session.active_group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: i64) -> UserId {
        UserId::new(id).unwrap()
    }

    #[test]
    fn creation_prompt_starts_and_can_be_repeated() {
        let sessions = BotSessions::new();
        let now = Instant::now();

        assert_eq!(
            sessions.begin_creation(user(1), now),
            BeginCreation::Started
        );
        assert_eq!(
            sessions.creation_state(user(1), now),
            CreationState::Awaiting
        );
        assert_eq!(
            sessions.begin_creation(user(1), now),
            BeginCreation::AlreadyPending
        );
    }

    #[test]
    fn creation_prompt_expires_after_ttl() {
        let sessions = BotSessions::new();
        let now = Instant::now();
        sessions.begin_creation(user(1), now);

        let later = now + CREATION_TTL + Duration::from_secs(1);
        assert_eq!(
            sessions.creation_state(user(1), later),
            CreationState::Expired
        );
        assert_eq!(
            sessions.creation_state(user(1), later),
            CreationState::NotStarted
        );
    }

    #[test]
    fn commands_can_cancel_pending_creation() {
        let sessions = BotSessions::new();
        let now = Instant::now();
        sessions.begin_creation(user(1), now);
        sessions.clear_creation(user(1));
        assert_eq!(
            sessions.creation_state(user(1), now),
            CreationState::NotStarted
        );
    }

    #[test]
    fn active_group_is_per_user() {
        let sessions = BotSessions::new();
        let a = GroupId::new(10).unwrap();
        let b = GroupId::new(20).unwrap();

        sessions.set_active_group(user(1), a);
        sessions.set_active_group(user(2), b);

        assert_eq!(sessions.active_group(user(1)), Some(a));
        assert_eq!(sessions.active_group(user(2)), Some(b));
        assert_eq!(sessions.active_group(user(3)), None);

        sessions.clear_active_group(user(1));
        assert_eq!(sessions.active_group(user(1)), None);
        assert_eq!(sessions.active_group(user(2)), Some(b));
    }
}
