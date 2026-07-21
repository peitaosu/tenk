use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use ratatui::text::{Line, Span};

use super::text::{display_width, truncate_display_fit};
use super::theme::UiTheme;

pub struct ListEntry<'a> {
    pub time: String,
    pub meta: &'a str,
    pub title: &'a str,
}

pub fn format_news_time_compact(publish_time: DateTime<Utc>) -> String {
    Local
        .from_utc_datetime(&publish_time.naive_utc())
        .format("%m-%d %H:%M")
        .to_string()
}

pub fn format_report_time_compact(publish_date: NaiveDate) -> String {
    publish_date.format("%m-%d").to_string()
}

pub fn format_institutional_time(research_date: NaiveDate) -> String {
    research_date.format("%m-%d").to_string()
}

pub fn format_report_time(publish_date: NaiveDate) -> String {
    format_report_time_compact(publish_date)
}

fn compact_head(time: &str, meta: &str, max_width: usize) -> String {
    let time = time.trim();
    if meta.is_empty() {
        return truncate_display_fit(&format!("{time} "), max_width);
    }
    let head = format!("{time} {meta} ");
    truncate_display_fit(&head, max_width)
}

fn compact_row_spans(
    time: &str,
    meta: &str,
    title: &str,
    width: usize,
    selected: bool,
    theme: UiTheme,
) -> Line<'static> {
    let head_max = (width * 2 / 5).max(10).min(width.saturating_sub(4));
    let head = compact_head(time, meta, head_max);
    let head_w = display_width(&head);
    let title_w = width.saturating_sub(head_w);
    let title = truncate_display_fit(title, title_w);

    if selected {
        let style = theme.list_selected();
        let pad = width.saturating_sub(head_w + display_width(&title));
        Line::from(vec![
            Span::styled(head, style),
            Span::styled(title, style),
            Span::styled(" ".repeat(pad), style),
        ])
    } else {
        Line::from(vec![
            Span::styled(head, theme.dim()),
            Span::styled(title, theme.text()),
        ])
    }
}

pub fn list_lines_compact(
    entries: &[ListEntry<'_>],
    scroll: usize,
    selected: usize,
    visible_h: usize,
    width: usize,
    theme: UiTheme,
) -> Vec<Line<'static>> {
    if width == 0 || visible_h == 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for (index, entry) in entries.iter().enumerate().skip(scroll) {
        if !lines.is_empty() && lines.len() >= visible_h {
            break;
        }
        lines.push(compact_row_spans(
            &entry.time,
            entry.meta,
            entry.title,
            width,
            index == selected,
            theme,
        ));
    }
    lines
}
