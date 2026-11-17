use chrono::{DateTime, Utc};

use crate::{
    domain::{GroupId, IanaTimezone, UserId},
    services::{
        calendar::{CalendarMember, GroupCalendar},
        timezone::{self, ReferenceWeek, TimeError},
    },
};

/// One ranked preference range in the viewer's reference week.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedInterval {
    pub day: u8,
    pub start_minute: u16,
    pub end_minute: u16,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// Configured members whose converted preference covers this range.
    pub member_ids: Vec<UserId>,
}

impl RankedInterval {
    pub fn count(&self) -> usize {
        self.member_ids.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestTime {
    pub group_id: GroupId,
    pub group_name: String,
    pub viewer_timezone: IanaTimezone,
    pub week: ReferenceWeek,
    pub total_member_count: usize,
    pub configured_member_count: usize,
    pub pending_member_count: usize,
    pub intervals: Vec<RankedInterval>,
}

/// Ranks converted preference intervals.
///
/// Boundaries come from the shared converted calendar, so the ranking uses the
/// same actual instants as the timetable. Each range is counted only when a
/// member's converted interval fully covers it. Pending members are excluded;
/// completed members with an empty calendar stay in the denominator and add no
/// votes.
pub fn rank(calendar: &GroupCalendar) -> Result<BestTime, TimeError> {
    let configured: Vec<&CalendarMember> = calendar
        .members
        .iter()
        .filter(|member| !member.setup_state.is_pending())
        .collect();
    let pending_member_count = calendar.members.len() - configured.len();

    let mut boundaries = vec![calendar.week.start, calendar.week.end];
    boundaries.extend(calendar.week.day_starts()?);
    for member in &configured {
        for segment in &member.segments {
            boundaries.push(segment.start);
            boundaries.push(segment.end);
        }
    }
    boundaries.sort();
    boundaries.dedup();

    let mut merged: Vec<RankedInterval> = Vec::new();

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start >= end {
            continue;
        }

        let covering: Vec<UserId> = configured
            .iter()
            .filter(|member| {
                member
                    .segments
                    .iter()
                    .any(|segment| segment.start <= start && segment.end >= end)
            })
            .map(|member| member.user_id)
            .collect();
        if covering.is_empty() {
            continue;
        }

        let (day, start_minute, end_minute) =
            timezone::viewer_position(start, end, &calendar.week)?;

        if let Some(previous) = merged.last_mut()
            && previous.day == day
            && previous.end == start
            && previous.end_minute == start_minute
            && previous.member_ids == covering
        {
            previous.end = end;
            previous.end_minute = end_minute;
            continue;
        }

        merged.push(RankedInterval {
            day,
            start_minute,
            end_minute,
            start,
            end,
            member_ids: covering,
        });
    }

    merged.sort_by(|left, right| {
        right
            .count()
            .cmp(&left.count())
            .then_with(|| left.start.cmp(&right.start))
    });

    Ok(BestTime {
        group_id: calendar.group_id,
        group_name: calendar.group_name.clone(),
        viewer_timezone: calendar.viewer_timezone,
        week: calendar.week.clone(),
        total_member_count: calendar.members.len(),
        configured_member_count: configured.len(),
        pending_member_count,
        intervals: merged,
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::{
        domain::{MemberRole, PreferenceInterval, SetupState, UserId, Weekday},
        services::{
            calendar::CalendarMember,
            timezone::{convert_schedule, reference_week},
        },
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 16, 10, 0, 0)
            .single()
            .unwrap()
    }

    fn week(viewer: &str) -> ReferenceWeek {
        reference_week(now(), IanaTimezone::parse(viewer).unwrap()).unwrap()
    }

    fn segments(
        week: &ReferenceWeek,
        tz: &str,
        weekday: Weekday,
        start: u16,
        end: u16,
    ) -> Vec<timezone::ConvertedSegment> {
        convert_schedule(
            &[PreferenceInterval::new(weekday, start, end).unwrap()],
            IanaTimezone::parse(tz).unwrap(),
            week,
        )
        .unwrap()
    }

    fn member(
        id: i64,
        setup_state: SetupState,
        segments: Vec<timezone::ConvertedSegment>,
    ) -> CalendarMember {
        CalendarMember {
            user_id: UserId::new(id).unwrap(),
            first_name: Some(format!("Member {id}")),
            last_name: None,
            username: None,
            role: MemberRole::Member,
            color: "#4DD0E1".to_owned(),
            timezone: None,
            setup_state,
            segments,
        }
    }

    fn calendar(members: Vec<CalendarMember>, week: ReferenceWeek) -> GroupCalendar {
        GroupCalendar {
            group_id: GroupId::new(1).unwrap(),
            group_name: "Test".to_owned(),
            viewer_timezone: week.timezone,
            week,
            members,
        }
    }

    #[test]
    fn ranks_covering_members_and_merges_equal_adjacent_ranges() {
        let week = week("Europe/Berlin");
        let a = member(
            1,
            SetupState::Completed,
            segments(&week, "Europe/Berlin", Weekday::Monday, 18 * 60, 20 * 60),
        );
        // Tbilisi 21:00 == Berlin 18:00 in November.
        let c = member(
            2,
            SetupState::Completed,
            segments(&week, "Asia/Tbilisi", Weekday::Monday, 21 * 60, 22 * 60),
        );
        let b = member(
            3,
            SetupState::Completed,
            segments(
                &week,
                "Europe/Berlin",
                Weekday::Monday,
                19 * 60 + 30,
                20 * 60,
            ),
        );
        // Completed-empty member stays in the denominator.
        let empty = member(4, SetupState::Completed, Vec::new());
        // Pending member contributes nothing even if data were present.
        let pending = member(
            5,
            SetupState::Pending,
            segments(&week, "Europe/Berlin", Weekday::Monday, 18 * 60, 20 * 60),
        );

        let ranked = rank(&calendar(vec![a, c, b, empty, pending], week)).unwrap();

        assert_eq!(ranked.total_member_count, 5);
        assert_eq!(ranked.configured_member_count, 4);
        assert_eq!(ranked.pending_member_count, 1);
        assert_eq!(ranked.intervals.len(), 3);

        assert_eq!(ranked.intervals[0].count(), 2);
        assert_eq!(ranked.intervals[0].start_minute, 18 * 60);
        assert_eq!(ranked.intervals[0].end_minute, 19 * 60);
        assert_eq!(
            ranked.intervals[0].member_ids,
            vec![UserId::new(1).unwrap(), UserId::new(2).unwrap()]
        );
        assert_eq!(ranked.intervals[1].count(), 2);
        assert_eq!(ranked.intervals[1].start_minute, 19 * 60 + 30);
        assert_eq!(ranked.intervals[1].end_minute, 20 * 60);
        assert_eq!(ranked.intervals[2].count(), 1);
        assert_eq!(ranked.intervals[2].start_minute, 19 * 60);
        assert_eq!(ranked.intervals[2].end_minute, 19 * 60 + 30);
        assert_eq!(
            ranked.intervals[2].member_ids,
            vec![UserId::new(1).unwrap()]
        );
    }

    #[test]
    fn identical_coverage_merges_into_one_range() {
        let week = week("Europe/Berlin");
        let a = member(
            1,
            SetupState::Completed,
            segments(&week, "Europe/Berlin", Weekday::Monday, 18 * 60, 20 * 60),
        );
        let b = member(
            2,
            SetupState::Completed,
            segments(&week, "Europe/Berlin", Weekday::Monday, 18 * 60, 20 * 60),
        );

        let ranked = rank(&calendar(vec![a, b], week)).unwrap();
        assert_eq!(ranked.intervals.len(), 1);
        assert_eq!(ranked.intervals[0].start_minute, 18 * 60);
        assert_eq!(ranked.intervals[0].end_minute, 20 * 60);
        assert_eq!(ranked.intervals[0].count(), 2);
        assert_eq!(
            ranked.intervals[0].member_ids,
            vec![UserId::new(1).unwrap(), UserId::new(2).unwrap()]
        );
    }

    #[test]
    fn completed_empty_members_do_not_create_intervals() {
        let week = week("Europe/Berlin");
        let empty = member(1, SetupState::Completed, Vec::new());
        let pending = member(2, SetupState::Pending, Vec::new());

        let ranked = rank(&calendar(vec![empty, pending], week)).unwrap();
        assert_eq!(ranked.configured_member_count, 1);
        assert_eq!(ranked.pending_member_count, 1);
        assert!(ranked.intervals.is_empty());
    }

    #[test]
    fn fractional_offsets_are_preserved_and_not_merged_across_days() {
        let week = week("Asia/Kathmandu");
        let a = member(
            1,
            SetupState::Completed,
            segments(&week, "Europe/Berlin", Weekday::Monday, 18 * 60, 19 * 60),
        );
        let b = member(
            2,
            SetupState::Completed,
            segments(&week, "Europe/Berlin", Weekday::Tuesday, 0, 30),
        );

        let ranked = rank(&calendar(vec![a, b], week)).unwrap();
        assert_eq!(ranked.intervals.len(), 2);
        assert_eq!(ranked.intervals[0].start_minute, 22 * 60 + 45);
        assert_eq!(ranked.intervals[0].end_minute, 23 * 60 + 45);
        assert_eq!(ranked.intervals[1].day, 1);
        assert_eq!(ranked.intervals[1].start_minute, 4 * 60 + 45);
        assert_eq!(ranked.intervals[1].end_minute, 5 * 60 + 15);
    }
}
