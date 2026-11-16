use chrono::{DateTime, Utc};

/// Injectable clock so reference-week and conversion behavior can be tested
/// deterministically.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FixedClock(DateTime<Utc>);

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self(now)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}
