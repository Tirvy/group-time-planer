use crate::domain::Weekday;

/// Boundary minute in a day, 0..=1440, on a 30-minute grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinuteOfDay(u16);

/// Recurring local interval `[start, end)` on one weekday.
///
/// The interval is half-open: `start` is included, `end` is excluded. This is
/// why `end_minute == 1440` (24:00) is valid while `start_minute == 1440` is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceInterval {
    weekday: Weekday,
    start: MinuteOfDay,
    end: MinuteOfDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreferenceError {
    #[error("minute {0} is outside 0..=1440")]
    MinuteOutOfRange(u16),
    #[error("minute {0} is not a multiple of 30")]
    NotHalfHour(u16),
    #[error("interval start must be before end")]
    EmptyInterval,
    #[error("intervals overlap on {0:?}")]
    Overlap(Weekday),
}

impl MinuteOfDay {
    pub const LAST_START: u16 = 1439;
    pub const END_OF_DAY: u16 = 1440;

    pub fn new(value: u16) -> Result<Self, PreferenceError> {
        if value > Self::END_OF_DAY {
            return Err(PreferenceError::MinuteOutOfRange(value));
        }
        if !value.is_multiple_of(30) {
            return Err(PreferenceError::NotHalfHour(value));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> u16 {
        self.0
    }
}

impl PreferenceInterval {
    pub fn new(
        weekday: Weekday,
        start_minute: u16,
        end_minute: u16,
    ) -> Result<Self, PreferenceError> {
        let start = MinuteOfDay::new(start_minute)?;
        let end = MinuteOfDay::new(end_minute)?;
        if start.value() >= end.value() {
            return Err(PreferenceError::EmptyInterval);
        }
        Ok(Self {
            weekday,
            start,
            end,
        })
    }

    pub fn weekday(self) -> Weekday {
        self.weekday
    }

    pub fn start_minute(self) -> u16 {
        self.start.value()
    }

    pub fn end_minute(self) -> u16 {
        self.end.value()
    }

    /// Half-open overlap: `[10:00, 11:00)` and `[11:00, 12:00)` do not overlap.
    pub fn overlaps(self, other: Self) -> bool {
        self.weekday == other.weekday
            && self.start.value() < other.end.value()
            && other.start.value() < self.end.value()
    }
}

/// Sorts intervals by weekday and start, then rejects any overlaps.
///
/// Adjacent intervals are kept separate; merging is a presentation concern.
pub fn normalize_intervals(
    mut intervals: Vec<PreferenceInterval>,
) -> Result<Vec<PreferenceInterval>, PreferenceError> {
    intervals.sort_by_key(|interval| {
        (
            interval.weekday.number(),
            interval.start.value(),
            interval.end.value(),
        )
    });

    let mut max_end: [Option<u16>; 7] = [None; 7];
    for interval in &intervals {
        let index = usize::from(interval.weekday.number()) - 1;
        if let Some(previous_end) = max_end[index]
            && interval.start.value() < previous_end
        {
            return Err(PreferenceError::Overlap(interval.weekday));
        }
        max_end[index] = Some(match max_end[index] {
            Some(previous_end) => previous_end.max(interval.end.value()),
            None => interval.end.value(),
        });
    }

    Ok(intervals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(weekday: Weekday, start: u16, end: u16) -> PreferenceInterval {
        PreferenceInterval::new(weekday, start, end).unwrap()
    }

    #[test]
    fn rejects_invalid_minutes() {
        assert_eq!(
            MinuteOfDay::new(1441),
            Err(PreferenceError::MinuteOutOfRange(1441))
        );
        assert_eq!(
            MinuteOfDay::new(615),
            Err(PreferenceError::NotHalfHour(615))
        );
        assert!(MinuteOfDay::new(1440).is_ok());
        assert!(MinuteOfDay::new(0).is_ok());
    }

    #[test]
    fn rejects_empty_or_inverted_intervals() {
        assert_eq!(
            PreferenceInterval::new(Weekday::Monday, 600, 600),
            Err(PreferenceError::EmptyInterval)
        );
        assert_eq!(
            PreferenceInterval::new(Weekday::Monday, 660, 600),
            Err(PreferenceError::EmptyInterval)
        );
    }

    #[test]
    fn accepts_full_day_boundary() {
        let full = interval(Weekday::Sunday, 0, 1440);
        assert_eq!(full.start_minute(), 0);
        assert_eq!(full.end_minute(), 1440);
    }

    #[test]
    fn half_open_overlap_semantics() {
        let morning = interval(Weekday::Monday, 600, 660);
        let adjacent = interval(Weekday::Monday, 660, 720);
        let overlapping = interval(Weekday::Monday, 630, 690);
        let other_day = interval(Weekday::Tuesday, 600, 660);

        assert!(!morning.overlaps(adjacent));
        assert!(morning.overlaps(overlapping));
        assert!(!morning.overlaps(other_day));
    }

    #[test]
    fn normalization_sorts_and_detects_nested_overlaps() {
        let sorted = normalize_intervals(vec![
            interval(Weekday::Tuesday, 660, 720),
            interval(Weekday::Monday, 600, 660),
        ])
        .unwrap();
        assert_eq!(sorted[0].weekday(), Weekday::Monday);
        assert_eq!(sorted[1].weekday(), Weekday::Tuesday);

        // The third interval overlaps the first one even though it is adjacent
        // to the second one; comparing only neighbours would miss this.
        let error = normalize_intervals(vec![
            interval(Weekday::Monday, 0, 120),
            interval(Weekday::Monday, 30, 60),
            interval(Weekday::Monday, 60, 90),
        ])
        .unwrap_err();
        assert_eq!(error, PreferenceError::Overlap(Weekday::Monday));
    }

    #[test]
    fn normalization_allows_adjacent_intervals() {
        let result = normalize_intervals(vec![
            interval(Weekday::Monday, 600, 660),
            interval(Weekday::Monday, 660, 720),
        ])
        .unwrap();
        assert_eq!(result.len(), 2);
    }
}
