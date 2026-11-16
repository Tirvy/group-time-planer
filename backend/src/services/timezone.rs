use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc,
};
use chrono_tz::Tz;

use crate::domain::{IanaTimezone, PreferenceInterval};

/// Viewer-relative week: Monday 00:00 through the next Monday 00:00 in the
/// viewer's timezone. The instant range can be 167 or 169 hours around DST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceWeek {
    pub timezone: IanaTimezone,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl ReferenceWeek {
    /// Viewer-local midnight instants for Monday through the following Monday.
    /// Used as hard boundaries when ranking or rendering.
    pub fn day_starts(&self) -> Result<Vec<DateTime<Utc>>, TimeError> {
        let mut starts = Vec::with_capacity(8);
        for offset in 0..=7 {
            let date = self
                .start_date
                .checked_add_signed(Duration::days(offset))
                .ok_or(TimeError::Overflow)?;
            starts.push(resolve_local(midnight(date)?, self.timezone.as_tz())?);
        }
        Ok(starts)
    }
}

/// A converted preference interval clipped to the reference week and split so
/// it never crosses viewer midnight or a viewer DST transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedSegment {
    /// Reference-week day index, Monday = 0 … Sunday = 6.
    pub day: u8,
    /// Minutes from viewer-local midnight.
    pub start_minute: u16,
    pub end_minute: u16,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum TimeError {
    #[error("cannot resolve local time {local} in {timezone}")]
    UnresolvableLocalTime {
        local: NaiveDateTime,
        timezone: String,
    },
    #[error("date/time arithmetic overflow")]
    Overflow,
}

/// Upcoming Monday–Sunday in the viewer's timezone. If the local day is Monday,
/// the current week starts today rather than next week.
pub fn reference_week(
    now: DateTime<Utc>,
    viewer: IanaTimezone,
) -> Result<ReferenceWeek, TimeError> {
    let tz = viewer.as_tz();
    let local_date = now.with_timezone(&tz).date_naive();
    let days_since_monday = i64::from(local_date.weekday().num_days_from_monday());
    let start_date = local_date - Duration::days(days_since_monday);
    let end_date = start_date + Duration::days(6);

    let start_local = midnight(start_date)?;
    let next_monday_local = midnight(start_date + Duration::days(7))?;
    let start = resolve_local(start_local, tz)?;
    let end = resolve_local(next_monday_local, tz)?;

    if end <= start {
        return Err(TimeError::Overflow);
    }

    Ok(ReferenceWeek {
        timezone: viewer,
        start_date,
        end_date,
        start,
        end,
    })
}

/// Expands recurring local intervals into actual reference-week instants, then
/// splits them by viewer-local day.
///
/// DST policy (docs/09): ambiguous local times use the earlier UTC offset;
/// nonexistent local times advance by the DST gap.
pub fn convert_schedule(
    intervals: &[PreferenceInterval],
    member_timezone: IanaTimezone,
    week: &ReferenceWeek,
) -> Result<Vec<ConvertedSegment>, TimeError> {
    let member_tz = member_timezone.as_tz();
    let viewer_tz = week.timezone.as_tz();

    let mut segments = Vec::new();
    let first_date = week.start.with_timezone(&member_tz).date_naive() - Duration::days(2);
    let last_date = week.end.with_timezone(&member_tz).date_naive() + Duration::days(2);

    let mut date = first_date;
    while date <= last_date {
        let weekday = date.weekday().num_days_from_monday() + 1;

        for interval in intervals {
            if u32::from(interval.weekday().number()) != weekday {
                continue;
            }

            let (start_local, end_local) = local_bounds(date, interval)?;
            let start = resolve_local(start_local, member_tz)?;
            let end = resolve_local(end_local, member_tz)?;
            if end <= start {
                continue;
            }

            let clipped_start = if start < week.start {
                week.start
            } else {
                start
            };
            let clipped_end = if end > week.end { week.end } else { end };
            if clipped_start >= clipped_end {
                continue;
            }

            for (piece_start, piece_end) in
                split_at_offset_changes(clipped_start, clipped_end, viewer_tz)
            {
                split_into_viewer_days(piece_start, piece_end, viewer_tz, week, &mut segments)?;
            }
        }

        date = date
            .checked_add_signed(Duration::days(1))
            .ok_or(TimeError::Overflow)?;
    }

    segments.sort_by_key(|segment| (segment.day, segment.start_minute, segment.start));
    Ok(segments)
}

/// Resolves a wall-clock time using the documented deterministic DST policy.
pub fn resolve_local(local: NaiveDateTime, tz: Tz) -> Result<DateTime<Utc>, TimeError> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(datetime) => Ok(datetime.with_timezone(&Utc)),
        LocalResult::Ambiguous(earliest, _latest) => Ok(earliest.with_timezone(&Utc)),
        LocalResult::None => {
            let next_offset = nearest_valid_offset(local, tz, 1)?;
            let previous_offset = nearest_valid_offset(local, tz, -1)?;
            let gap_seconds = next_offset - previous_offset;
            if gap_seconds <= 0 {
                return Err(unresolvable(local, tz));
            }

            let shifted = local
                .checked_add_signed(Duration::seconds(i64::from(gap_seconds)))
                .ok_or(TimeError::Overflow)?;

            match tz.from_local_datetime(&shifted) {
                LocalResult::Single(datetime) => Ok(datetime.with_timezone(&Utc)),
                LocalResult::Ambiguous(earliest, _latest) => Ok(earliest.with_timezone(&Utc)),
                LocalResult::None => Err(unresolvable(local, tz)),
            }
        }
    }
}

/// Converts an instant interval back into viewer-local day and minute bounds.
pub fn viewer_position(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    week: &ReferenceWeek,
) -> Result<(u8, u16, u16), TimeError> {
    if end <= start {
        return Err(TimeError::Overflow);
    }

    let tz = week.timezone.as_tz();
    let local_start = start.with_timezone(&tz);
    let date = local_start.date_naive();
    let day = (date - week.start_date).num_days();
    if !(0..7).contains(&day) {
        return Err(TimeError::Overflow);
    }

    let local_midnight = midnight(date)?;
    let start_minute = (local_start.naive_local() - local_midnight).num_minutes();
    let probe = end - Duration::seconds(1);
    let local_end = probe.with_timezone(&tz);
    let end_minute = if local_end.date_naive() != date {
        1440
    } else {
        let seconds = (local_end.naive_local() - local_midnight).num_seconds();
        ((seconds.max(0) + 59) / 60).min(1440)
    };

    if end_minute <= start_minute {
        return Err(TimeError::Overflow);
    }

    Ok((
        day as u8,
        u16::try_from(start_minute).map_err(|_| TimeError::Overflow)?,
        u16::try_from(end_minute).map_err(|_| TimeError::Overflow)?,
    ))
}

fn nearest_valid_offset(local: NaiveDateTime, tz: Tz, direction: i64) -> Result<i32, TimeError> {
    let mut probe = local;
    let mut steps = 0;
    while steps < 48 * 60 {
        probe = probe
            .checked_add_signed(Duration::minutes(direction))
            .ok_or(TimeError::Overflow)?;
        match tz.from_local_datetime(&probe) {
            LocalResult::Single(datetime) | LocalResult::Ambiguous(datetime, _) => {
                return Ok(datetime.offset().fix().local_minus_utc());
            }
            LocalResult::None => {}
        }
        steps += 1;
    }
    Err(unresolvable(local, tz))
}

fn unresolvable(local: NaiveDateTime, tz: Tz) -> TimeError {
    TimeError::UnresolvableLocalTime {
        local,
        timezone: tz.name().to_owned(),
    }
}

fn midnight(date: NaiveDate) -> Result<NaiveDateTime, TimeError> {
    date.and_hms_opt(0, 0, 0).ok_or(TimeError::Overflow)
}

fn local_bounds(
    date: NaiveDate,
    interval: &PreferenceInterval,
) -> Result<(NaiveDateTime, NaiveDateTime), TimeError> {
    let base = midnight(date)?;
    let start = base
        .checked_add_signed(Duration::minutes(i64::from(interval.start_minute())))
        .ok_or(TimeError::Overflow)?;

    let end = if interval.end_minute() == 1440 {
        midnight(
            date.checked_add_signed(Duration::days(1))
                .ok_or(TimeError::Overflow)?,
        )?
    } else {
        base.checked_add_signed(Duration::minutes(i64::from(interval.end_minute())))
            .ok_or(TimeError::Overflow)?
    };

    Ok((start, end))
}

fn local_offset(datetime: DateTime<Utc>, tz: Tz) -> i32 {
    datetime.with_timezone(&tz).offset().fix().local_minus_utc()
}

/// Splits at offset changes so a segment never crosses a DST transition. This
/// keeps wall-clock start/end ordered even on a fall-back day.
fn split_at_offset_changes(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    tz: Tz,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut pieces = Vec::new();
    let mut cursor = start;

    while cursor < end {
        let cursor_offset = local_offset(cursor, tz);
        let end_probe = end - Duration::seconds(1);
        if cursor_offset == local_offset(end_probe, tz) {
            pieces.push((cursor, end));
            break;
        }

        let mut low = cursor;
        let mut high = end;
        while (high - low) > Duration::seconds(1) {
            let mid = low + Duration::seconds((high - low).num_seconds() / 2);
            if local_offset(mid, tz) == cursor_offset {
                low = mid;
            } else {
                high = mid;
            }
        }

        if high <= cursor || high >= end {
            pieces.push((cursor, end));
            break;
        }

        pieces.push((cursor, high));
        cursor = high;
    }

    pieces
}

fn split_into_viewer_days(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    viewer_tz: Tz,
    week: &ReferenceWeek,
    output: &mut Vec<ConvertedSegment>,
) -> Result<(), TimeError> {
    let mut cursor = start;

    while cursor < end {
        let local = cursor.with_timezone(&viewer_tz);
        let date = local.date_naive();
        let local_midnight = midnight(date)?;
        let next_date = date
            .checked_add_signed(Duration::days(1))
            .ok_or(TimeError::Overflow)?;
        let next_midnight_local = midnight(next_date)?;
        let next_midnight = resolve_local(next_midnight_local, viewer_tz)?;
        let segment_end = if end < next_midnight {
            end
        } else {
            next_midnight
        };

        if segment_end <= cursor {
            return Err(TimeError::Overflow);
        }

        let start_minute = (local.naive_local() - local_midnight).num_minutes();
        let end_minute = if segment_end == next_midnight {
            1440
        } else {
            // A piece can end exactly at a fall-back transition, where the
            // boundary instant already has the new offset. Use the last wall
            // second inside the piece and round up to the next minute.
            let probe = segment_end - Duration::seconds(1);
            let probe_local = probe.with_timezone(&viewer_tz);
            let seconds = (probe_local.naive_local() - local_midnight).num_seconds();
            ((seconds.max(0) + 59) / 60).min(1440)
        };

        let day = (date - week.start_date).num_days();
        if (0..7).contains(&day) && end_minute > start_minute {
            output.push(ConvertedSegment {
                day: day as u8,
                start_minute: u16::try_from(start_minute).map_err(|_| TimeError::Overflow)?,
                end_minute: u16::try_from(end_minute).map_err(|_| TimeError::Overflow)?,
                start: cursor,
                end: segment_end,
            });
        }

        cursor = segment_end;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::domain::Weekday;

    fn utc(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
    }

    fn tz(name: &str) -> IanaTimezone {
        IanaTimezone::parse(name).unwrap()
    }

    fn interval(weekday: Weekday, start: u16, end: u16) -> PreferenceInterval {
        PreferenceInterval::new(weekday, start, end).unwrap()
    }

    #[test]
    fn reference_week_starts_on_monday_and_monday_uses_current_week() {
        let viewer = tz("Asia/Tbilisi");
        let monday = reference_week(utc(2026, 11, 16, 8, 0), viewer).unwrap();
        assert_eq!(
            monday.start_date,
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
        assert_eq!(
            monday.end_date,
            NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
        );

        let sunday = reference_week(utc(2026, 11, 22, 12, 0), viewer).unwrap();
        assert_eq!(
            sunday.start_date,
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );

        let tuesday = reference_week(utc(2026, 11, 17, 12, 0), viewer).unwrap();
        assert_eq!(
            tuesday.start_date,
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
    }

    #[test]
    fn reference_week_uses_viewer_local_day() {
        // Sunday 22:00 UTC is already Monday 02:00 in Tbilisi.
        let week = reference_week(utc(2026, 11, 15, 22, 0), tz("Asia/Tbilisi")).unwrap();
        assert_eq!(
            week.start_date,
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
    }

    #[test]
    fn reference_week_length_follows_dst() {
        let normal = reference_week(utc(2026, 11, 16, 12, 0), tz("Europe/Berlin")).unwrap();
        assert_eq!(
            (normal.end - normal.start).num_hours(),
            7 * 24,
            "normal week is 168 hours"
        );

        // Berlin spring forward on 2026-03-29.
        let dst = reference_week(utc(2026, 3, 24, 12, 0), tz("Europe/Berlin")).unwrap();
        assert_eq!(
            dst.start_date,
            NaiveDate::from_ymd_opt(2026, 3, 23).unwrap()
        );
        assert_eq!(
            (dst.end - dst.start).num_hours(),
            7 * 24 - 1,
            "spring-forward week is 167 hours"
        );
    }

    #[test]
    fn ambiguous_local_time_uses_first_occurrence() {
        // Berlin fall-back on 2026-10-25: 02:30 occurs twice.
        let local = NaiveDate::from_ymd_opt(2026, 10, 25)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let resolved = resolve_local(local, tz("Europe/Berlin").as_tz()).unwrap();
        assert_eq!(resolved, utc(2026, 10, 25, 0, 30));
    }

    #[test]
    fn nonexistent_local_time_advances_by_the_gap() {
        // Berlin spring-forward on 2026-03-29: 02:30 does not exist.
        let local = NaiveDate::from_ymd_opt(2026, 3, 29)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let resolved = resolve_local(local, tz("Europe/Berlin").as_tz()).unwrap();
        // 03:30 CEST == 01:30 UTC.
        assert_eq!(resolved, utc(2026, 3, 29, 1, 30));
    }

    #[test]
    fn handles_a_thirty_minute_dst_gap() {
        // Lord Howe spring-forward on 2026-10-04: 02:00 -> 02:30.
        let local = NaiveDate::from_ymd_opt(2026, 10, 4)
            .unwrap()
            .and_hms_opt(2, 15, 0)
            .unwrap();
        let resolved = resolve_local(local, tz("Australia/Lord_Howe").as_tz()).unwrap();
        // 02:45 +11:00 == 15:45 UTC on the previous day.
        assert_eq!(resolved, utc(2026, 10, 3, 15, 45));
    }

    #[test]
    fn converts_member_intervals_into_viewer_timezone() {
        let week = reference_week(utc(2026, 11, 16, 10, 0), tz("Europe/Berlin")).unwrap();
        // 21:00 Tbilisi (UTC+4) == 18:00 Berlin (UTC+1) in November.
        let segments = convert_schedule(
            &[interval(Weekday::Monday, 21 * 60, 22 * 60)],
            tz("Asia/Tbilisi"),
            &week,
        )
        .unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].day, 0);
        assert_eq!(segments[0].start_minute, 18 * 60);
        assert_eq!(segments[0].end_minute, 19 * 60);
        assert_eq!(segments[0].start, utc(2026, 11, 16, 17, 0));
    }

    #[test]
    fn clips_intervals_to_the_viewer_week_and_splits_midnight() {
        let week = reference_week(utc(2026, 11, 16, 10, 0), tz("UTC")).unwrap();
        // Full Tuesday in Tokyo == Monday 15:00 UTC through Tuesday 15:00 UTC.
        let segments = convert_schedule(
            &[interval(Weekday::Tuesday, 0, 1440)],
            tz("Asia/Tokyo"),
            &week,
        )
        .unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].day, 0);
        assert_eq!(segments[0].start_minute, 15 * 60);
        assert_eq!(segments[0].end_minute, 1440);
        assert_eq!(segments[1].day, 1);
        assert_eq!(segments[1].start_minute, 0);
        assert_eq!(segments[1].end_minute, 15 * 60);
    }

    #[test]
    fn supports_non_half_hour_offsets() {
        let week = reference_week(utc(2026, 11, 16, 10, 0), tz("Asia/Kathmandu")).unwrap();
        // 18:00 Berlin == 22:45 Kathmandu (UTC+5:45).
        let segments = convert_schedule(
            &[interval(Weekday::Monday, 18 * 60, 19 * 60)],
            tz("Europe/Berlin"),
            &week,
        )
        .unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_minute, 22 * 60 + 45);
        assert_eq!(segments[0].end_minute, 23 * 60 + 45);
    }

    #[test]
    fn splits_at_fall_back_so_wall_clock_ranges_stay_ordered() {
        // UTC member interval 00:30–01:30 on the Berlin fall-back Sunday.
        let week = reference_week(utc(2026, 10, 20, 10, 0), tz("Europe/Berlin")).unwrap();
        let segments =
            convert_schedule(&[interval(Weekday::Sunday, 30, 90)], tz("UTC"), &week).unwrap();

        assert_eq!(segments.len(), 2);
        for segment in &segments {
            assert!(
                segment.end_minute > segment.start_minute,
                "wall clock ranges must stay ordered: {segment:?}"
            );
        }
    }
}
