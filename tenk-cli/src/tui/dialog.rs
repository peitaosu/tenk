use chrono::{Local, TimeZone};
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;
use rust_i18n::t;
use tenk::ResearchReportData;

use super::app::Dialog;
use super::list::format_report_time;
use super::text::wrap_display;
use super::theme::UiTheme;

pub fn render_dialog(frame: &mut Frame, dialog: &Dialog, theme: UiTheme) {
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);

    match dialog {
        Dialog::News {
            loading,
            scroll,
            content,
        } => render_news_dialog(frame, area, *loading, *scroll, content.as_ref(), theme),
        Dialog::Research(report) => render_research_dialog(frame, area, report, theme),
    }
}

fn render_news_dialog(
    frame: &mut Frame,
    area: Rect,
    loading: bool,
    scroll: usize,
    content: Option<&tenk::NewsContent>,
    theme: UiTheme,
) {
    let block = theme.panel_block(&t!("tui.dialog.news"), true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if loading {
        let p = Paragraph::new(t!("tui.status.loading"))
            .style(theme.dim())
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    let Some(content) = content else {
        let p = Paragraph::new(t!("messages.no_data"))
            .style(theme.dim())
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    };

    let local_time = Local
        .from_utc_datetime(&content.publish_time.naive_utc())
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let meta = format!(
        "{} · {} · {}",
        content.source,
        local_time,
        content
            .author
            .as_deref()
            .unwrap_or("")
    );

    let width = inner.width as usize;
    let mut lines: Vec<Line> = Vec::new();
    for line in wrap_display(&content.title, width) {
        lines.push(Line::from(Span::styled(line, theme.section())));
    }
    for line in wrap_display(&meta, width) {
        lines.push(Line::from(Span::styled(line, theme.dim())));
    }
    lines.push(Line::from(""));
    for line in wrap_display(&content.body_text, width) {
        lines.push(Line::from(Span::styled(line, theme.text())));
    }

    let visible: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(inner.height as usize)
        .collect();
    frame.render_widget(Paragraph::new(Text::from(visible)), inner);
}

fn render_research_dialog(frame: &mut Frame, area: Rect, report: &ResearchReportData, theme: UiTheme) {
    let block = theme.panel_block(&t!("tui.dialog.research"), true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rating = report
        .rating
        .as_deref()
        .unwrap_or("-");
    let width = inner.width as usize;
    let mut lines: Vec<Line> = Vec::new();
    for line in wrap_display(&report.title, width) {
        lines.push(Line::from(Span::styled(line, theme.section())));
    }
    for line in wrap_display(
        &format!(
            "{} · {} · {}",
            report.institution,
            format_report_time(report.publish_date).trim(),
            rating
        ),
        width,
    ) {
        lines.push(Line::from(Span::styled(line, theme.dim())));
    }
    for line in wrap_display(
        &format!("{}: {}", t!("headers.code"), report.stock_code),
        width,
    ) {
        lines.push(Line::from(Span::styled(line, theme.text())));
    }
    for line in wrap_display(
        &format!("{}: {}", t!("headers.name"), report.stock_name),
        width,
    ) {
        lines.push(Line::from(Span::styled(line, theme.text())));
    }
    for line in wrap_display(
        &format!("{}: {}", t!("tui.labels.analysts"), report.analysts),
        width,
    ) {
        lines.push(Line::from(Span::styled(line, theme.text())));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t!("tui.dialog.no_report_body"),
        theme.dim(),
    )));
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

pub fn render_help(frame: &mut Frame, theme: UiTheme) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);
    let lines: Vec<Line> = t!("tui.help")
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), theme.text())))
        .collect();
    let block = theme.panel_block(&t!("tui.help_title"), true);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub fn render_command_line(frame: &mut Frame, input: &str, theme: UiTheme) {
    let area = Rect {
        x: frame.area().x,
        y: frame.area().height.saturating_sub(1),
        width: frame.area().width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(": ", theme.highlight()),
            Span::styled(input, theme.text()),
            Span::styled("_", theme.highlight()),
        ])),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
