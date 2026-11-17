use std::{
    fmt::Write as _,
    sync::{Arc, OnceLock},
};

use chrono::{DateTime, Duration, Utc};

use crate::services::calendar::{CalendarMember, GroupCalendar};

const WIDTH: u32 = 700;
const MARGIN: f32 = 28.0;
const HEADER_H: f32 = 126.0;
const GRID_TOP: f32 = HEADER_H + 18.0;
const DAY_HEADER_H: f32 = 40.0;
const ROW_MINUTES: u16 = 30;
const ROWS_PER_DAY: usize = 24 * 60 / ROW_MINUTES as usize;
const ROW_H: f32 = 16.0;
const TIME_W: f32 = 52.0;
const DAY_W: f32 = (WIDTH as f32 - 2.0 * MARGIN - TIME_W) / 7.0;
const GRID_H: f32 = ROWS_PER_DAY as f32 * ROW_H;
const LEGEND_GAP: f32 = 30.0;
const LEGEND_ROW_H: f32 = 22.0;
const FOOTER_H: f32 = 80.0;
const DAY_SHORT: [&str; 7] = ["ПН", "ВТ", "СР", "ЧТ", "ПТ", "СБ", "ВС"];

const BG: &str = "#0b0f19";
const SURFACE: &str = "#141b2b";
const SURFACE_ALT: &str = "#101725";
const LINE: &str = "#273246";
const LINE_STRONG: &str = "#34435c";
const TEXT: &str = "#e8eefc";
const MUTED: &str = "#93a4bd";
const WARNING: &str = "#ffb74d";

#[derive(Debug, Clone)]
pub struct TimetableSvg {
    pub svg: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum TimetableError {
    #[error("failed to parse generated SVG: {0}")]
    Svg(#[from] resvg::usvg::Error),
    #[error("failed to allocate PNG pixmap")]
    Pixmap,
    #[error("failed to encode PNG: {0}")]
    Encode(String),
}

/// Builds the SVG timetable from the same converted calendar model used by the
/// interactive combined view.
pub fn render_svg(calendar: &GroupCalendar, now: DateTime<Utc>) -> TimetableSvg {
    let legend_rows = calendar.members.len().div_ceil(2) as f32;
    let height = GRID_TOP
        + DAY_HEADER_H
        + GRID_H
        + DAY_HEADER_H
        + LEGEND_GAP
        + 24.0
        + legend_rows * LEGEND_ROW_H
        + FOOTER_H;

    let mut svg = String::with_capacity(96 * 1024);
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" viewBox="0 0 {WIDTH} {height}">"#
    );
    let _ = write!(
        svg,
        r#"<rect x="0" y="0" width="{WIDTH}" height="{height}" fill="{BG}"/>"#
    );

    // Header
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="42" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="26" font-weight="700" fill="{TEXT}">{}</text>"#,
        escape(&calendar.group_name)
    );
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="72" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="14" fill="{MUTED}">Неделя: {} — {}</text>"#,
        format_date(calendar.week.start_date),
        format_date(calendar.week.end_date)
    );
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="94" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="14" fill="{MUTED}">Часовой пояс: {}</text>"#,
        escape(calendar.viewer_timezone.name())
    );
    let generated = now
        .with_timezone(&calendar.viewer_timezone.as_tz())
        .format("%d.%m.%Y %H:%M");
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="116" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="12" fill="{MUTED}">Сформировано: {generated}</text>"#
    );

    let grid_left = MARGIN + TIME_W;
    let grid_top = GRID_TOP + DAY_HEADER_H;

    // Day headers: weekday and date on separate centered rows.
    write_day_headers(&mut svg, calendar, GRID_TOP);

    // Grid background + lines
    let _ = write!(
        svg,
        r#"<rect x="{grid_left}" y="{grid_top}" width="{}" height="{GRID_H}" fill="{SURFACE_ALT}"/>"#,
        DAY_W * 7.0
    );

    for line in 0..=ROWS_PER_DAY {
        let y = grid_top + line as f32 * ROW_H;
        let color = if line % 2 == 0 { LINE_STRONG } else { LINE };
        let width = if line % 2 == 0 { 1.0 } else { 0.5 };
        let _ = write!(
            svg,
            r#"<line x1="{grid_left}" y1="{y}" x2="{}" y2="{y}" stroke="{color}" stroke-width="{width}"/>"#,
            grid_left + DAY_W * 7.0
        );
    }
    for day in 0..=7 {
        let x = grid_left + day as f32 * DAY_W;
        let _ = write!(
            svg,
            r#"<line x1="{x}" y1="{grid_top}" x2="{x}" y2="{}" stroke="{LINE}" stroke-width="1"/>"#,
            grid_top + GRID_H
        );
    }

    // Time labels every hour
    let rows_per_hour = 60 / ROW_MINUTES as usize;
    for hour in 0..24 {
        let row = hour * rows_per_hour;
        let y = grid_top + row as f32 * ROW_H + ROW_H - 1.0;
        let _ = write!(
            svg,
            r#"<text x="{}" y="{y}" text-anchor="end" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="9" fill="{MUTED}">{hour:02}:00</text>"#,
            MARGIN + TIME_W - 7.0
        );
    }

    // Member dots by 30-minute slice
    let mut cells: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); ROWS_PER_DAY]; 7];
    for (index, member) in calendar.members.iter().enumerate() {
        for segment in &member.segments {
            let first = usize::from(segment.start_minute / ROW_MINUTES);
            let last = usize::from(segment.end_minute.div_ceil(ROW_MINUTES)).min(ROWS_PER_DAY);
            for slot in first..last {
                if let Some(cell) = cells
                    .get_mut(usize::from(segment.day))
                    .and_then(|day| day.get_mut(slot))
                {
                    cell.push(index);
                }
            }
        }
    }

    for (day, day_cells) in cells.iter().enumerate() {
        for (slot, members) in day_cells.iter().enumerate() {
            if members.is_empty() {
                continue;
            }
            let cell_x = grid_left + day as f32 * DAY_W;
            let cell_y = grid_top + slot as f32 * ROW_H;
            for (index, member_index) in members.iter().enumerate() {
                let x = cell_x + 4.0 + (index % 13) as f32 * 6.0;
                let y = cell_y + 4.0 + (index / 13) as f32 * 5.0;
                let color = &calendar.members[*member_index].color;
                let _ = write!(
                    svg,
                    r#"<circle cx="{x}" cy="{y}" r="2.8" fill="{}"/>"#,
                    escape(color)
                );
            }
        }
    }

    // Legend
    write_day_headers(&mut svg, calendar, grid_top + GRID_H);

    let legend_top = grid_top + GRID_H + DAY_HEADER_H + LEGEND_GAP;
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="{legend_top}" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="14" font-weight="700" fill="{TEXT}">Участники</text>"#
    );

    let column_width = (WIDTH as f32 - 2.0 * MARGIN) / 2.0;
    for (index, member) in calendar.members.iter().enumerate() {
        let column = index % 2;
        let row = index / 2;
        let x = MARGIN + column as f32 * column_width;
        let y = legend_top + 18.0 + row as f32 * LEGEND_ROW_H;
        let _ = write!(
            svg,
            r#"<rect x="{x}" y="{}" width="10" height="10" rx="4" fill="{}"/>"#,
            y - 8.0,
            escape(&member.color)
        );
        let _ = write!(
            svg,
            r#"<text x="{}" y="{y}" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="11" fill="{TEXT}">{}</text>"#,
            x + 16.0,
            escape(&truncate(&member_name(member), 18))
        );
        let role = if member.role.is_owner() {
            "владелец"
        } else {
            "участник"
        };
        let role = if member.setup_state.is_pending() {
            format!("{role} · календарь не настроен")
        } else {
            role.to_owned()
        };
        let role_color = if member.setup_state.is_pending() {
            WARNING
        } else {
            MUTED
        };
        let _ = write!(
            svg,
            r#"<text x="{}" y="{y}" text-anchor="end" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="10" fill="{role_color}">{role}</text>"#,
            x + column_width - 10.0
        );
    }

    // Footer
    let footer_y = height - 56.0;
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="{footer_y}" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="10" fill="{MUTED}">Отмеченное время означает предпочтение, а не подтверждённую доступность.</text>"#
    );
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="{}" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="10" fill="{MUTED}">Участники без настроенного календаря отмечены в легенде;</text>"#,
        footer_y + 16.0
    );
    let _ = write!(
        svg,
        r#"<text x="{MARGIN}" y="{}" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="10" fill="{MUTED}">пустая клетка не означает недоступность.</text>"#,
        footer_y + 32.0
    );
    svg.push_str("</svg>");

    TimetableSvg {
        svg,
        width: WIDTH,
        height: height as u32,
    }
}

/// Renders the SVG timetable to PNG with `resvg`. System fonts are loaded so
/// Russian labels render on the deployment host.
pub fn render_png(calendar: &GroupCalendar, now: DateTime<Utc>) -> Result<Vec<u8>, TimetableError> {
    let timetable = render_svg(calendar, now);

    let options = resvg::usvg::Options {
        fontdb: font_database(),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(&timetable.svg, &options)?;

    let scale = 2u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(timetable.width * scale, timetable.height * scale)
            .ok_or(TimetableError::Pixmap)?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(2.0, 2.0),
        &mut pixmap.as_mut(),
    );

    pixmap
        .encode_png()
        .map_err(|error| TimetableError::Encode(error.to_string()))
}

fn write_day_headers(svg: &mut String, calendar: &GroupCalendar, top: f32) {
    let grid_left = MARGIN + TIME_W;
    for (day, short) in DAY_SHORT.iter().enumerate() {
        let x = grid_left + day as f32 * DAY_W;
        let center_x = x + DAY_W / 2.0;
        let date = calendar.week.start_date + Duration::days(day as i64);
        let _ = write!(
            svg,
            r#"<rect x="{x}" y="{top}" width="{DAY_W}" height="{DAY_HEADER_H}" fill="{SURFACE}"/>"#
        );
        let _ = write!(
            svg,
            r#"<text x="{center_x}" y="{}" text-anchor="middle" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" fill="{TEXT}">{short}</text>"#,
            top + 17.0
        );
        let _ = write!(
            svg,
            r#"<text x="{center_x}" y="{}" text-anchor="middle" font-family="DejaVu Sans, Segoe UI, Arial, sans-serif" font-size="11" fill="{MUTED}">{}</text>"#,
            top + 32.0,
            date.format("%d.%m")
        );
    }
}

fn font_database() -> Arc<resvg::usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = resvg::usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn member_name(member: &CalendarMember) -> String {
    let first = member.first_name.as_deref().unwrap_or("").trim();
    let last = member.last_name.as_deref().unwrap_or("").trim();

    if !first.is_empty() && !last.is_empty() {
        format!("{first} {last}")
    } else if !first.is_empty() {
        first.to_owned()
    } else if let Some(username) = member.username.as_deref() {
        format!("@{username}")
    } else {
        "Участник".to_owned()
    }
}

fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%d.%m.%Y").to_string()
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::{
        domain::{GroupId, IanaTimezone, MemberRole, SetupState, UserId},
        services::{
            calendar::CalendarMember,
            timezone::{convert_schedule, reference_week},
        },
    };

    fn calendar() -> GroupCalendar {
        let week = reference_week(
            Utc.with_ymd_and_hms(2026, 11, 16, 10, 0, 0)
                .single()
                .unwrap(),
            IanaTimezone::parse("Europe/Berlin").unwrap(),
        )
        .unwrap();
        let segments = convert_schedule(
            &[crate::domain::PreferenceInterval::new(
                crate::domain::Weekday::Monday,
                18 * 60,
                19 * 60,
            )
            .unwrap()],
            IanaTimezone::parse("Europe/Berlin").unwrap(),
            &week,
        )
        .unwrap();

        GroupCalendar {
            group_id: GroupId::new(1).unwrap(),
            group_name: "Рейд & Ко".to_owned(),
            viewer_timezone: week.timezone,
            week,
            members: vec![
                CalendarMember {
                    user_id: UserId::new(1).unwrap(),
                    first_name: Some("Иван".to_owned()),
                    last_name: Some("Петров".to_owned()),
                    username: None,
                    role: MemberRole::Owner,
                    color: "#4DD0E1".to_owned(),
                    setup_state: SetupState::Completed,
                    segments,
                },
                CalendarMember {
                    user_id: UserId::new(2).unwrap(),
                    first_name: Some("Пётр".to_owned()),
                    last_name: None,
                    username: None,
                    role: MemberRole::Member,
                    color: "#7E57C2".to_owned(),
                    setup_state: SetupState::Pending,
                    segments: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn svg_contains_identity_week_legend_and_pending_status() {
        let svg = render_svg(&calendar(), Utc::now()).svg;
        assert!(svg.contains("Рейд &amp; Ко"));
        assert!(svg.contains("16.11.2026 — 22.11.2026"));
        assert!(svg.contains("Europe/Berlin"));
        assert!(svg.contains("Иван Петров"));
        assert!(svg.contains("Пётр"));
        assert!(svg.contains("календарь не настроен"));
        assert!(svg.contains(">ПН</text>"));
        assert_eq!(
            svg.matches(">ПН</text>").count(),
            2,
            "day header repeated at the bottom"
        );
        assert!(svg.contains(">16.11</text>"));
        assert!(svg.contains("Пустая клетка") || svg.contains("пустая клетка"));
    }

    #[test]
    fn png_has_valid_signature_and_expected_dimensions() {
        let png = render_png(&calendar(), Utc::now()).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        assert_eq!(width, WIDTH * 2);
        assert!(height > 1000, "timetable should be tall");
    }
}
