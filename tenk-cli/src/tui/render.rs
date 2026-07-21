use chrono::Local;
use chandelier::{
    CandleSeries, CandlestickChart, LineChart, LineSeries, PriceAxis, TimeAxis, TrendLine, ValueAxis,
    VolumeChart, VolumeSeries,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;
use rust_i18n::t;

use crate::i18n::format_volume_compact;

use super::app::{App, Focus, Overlay, StockSearch};
use super::feed::{advice_label, FeedData};
use super::dialog::{render_command_line, render_dialog, render_help};
use super::kline::{self, KlineView};
use super::list::{format_institutional_time, format_news_time_compact, format_report_time_compact, list_lines_compact, ListEntry};
use super::quote::{quote_grid_lines, quote_rows};
use super::style::{format_change_pct, format_price};
use super::theme::UiTheme;
use super::text::{pad_display_end};

const WATCHLIST_GAP: usize = 2;
const WATCHLIST_MARKER_W: usize = 2;
const WATCHLIST_CODE_W: usize = 8;
const WATCHLIST_PRICE_W: usize = 10;
const WATCHLIST_CHG_W: usize = 9;
const SEARCH_FIELD_HEIGHT: u16 = 3;
const SEARCH_CODE_W: usize = 8;
const SEARCH_MARKET_W: usize = 8;

struct WatchlistColumns {
    marker_w: usize,
    code_w: usize,
    name_w: usize,
    price_w: usize,
    chg_w: usize,
    gap: usize,
}

fn watchlist_columns(width: usize) -> WatchlistColumns {
    let fixed = WATCHLIST_MARKER_W
        + WATCHLIST_CODE_W
        + WATCHLIST_PRICE_W
        + WATCHLIST_CHG_W
        + WATCHLIST_GAP * 4;
    WatchlistColumns {
        marker_w: WATCHLIST_MARKER_W,
        code_w: WATCHLIST_CODE_W,
        name_w: width.saturating_sub(fixed).max(4),
        price_w: WATCHLIST_PRICE_W,
        chg_w: WATCHLIST_CHG_W,
        gap: WATCHLIST_GAP,
    }
}

fn watchlist_row_spans(
    marker: &str,
    code: &str,
    name: &str,
    price: &str,
    chg: &str,
    cols: &WatchlistColumns,
    marker_style: Style,
    name_style: Style,
    value_style: Style,
) -> Vec<Span<'static>> {
    vec![
        Span::styled(pad_display_end(marker, cols.marker_w), marker_style),
        Span::raw(" ".repeat(cols.gap)),
        Span::styled(pad_display_end(code, cols.code_w), marker_style),
        Span::raw(" ".repeat(cols.gap)),
        Span::styled(pad_display_end(name, cols.name_w), name_style),
        Span::raw(" ".repeat(cols.gap)),
        Span::styled(pad_display_end(price, cols.price_w), value_style),
        Span::raw(" ".repeat(cols.gap)),
        Span::styled(pad_display_end(chg, cols.chg_w), value_style),
    ]
}

pub fn render(frame: &mut Frame, app: &App) {
    let theme = app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(frame.area().height.saturating_sub(1)),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(5, 8), Constraint::Ratio(3, 8)])
        .split(chunks[0]);

    let upper = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(3, 10), Constraint::Ratio(7, 10)])
        .split(main[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(7), Constraint::Min(4)])
        .split(upper[1]);

    render_watchlist(frame, upper[0], app, theme);
    render_quote(frame, right[0], app, theme);
    render_kline(frame, right[1], app, theme);
    render_feeds(frame, main[1], app, theme);
    render_footer(frame, chunks[1], app, theme);

    if let Some(ref dialog) = app.dialog {
        render_dialog(frame, dialog, theme);
    }
    if matches!(app.overlay, Overlay::Help) {
        render_help(frame, theme);
    }
    if let Overlay::Command(ref input) = app.overlay {
        render_command_line(frame, input, theme);
    }
}

fn render_watchlist(frame: &mut Frame, area: Rect, app: &App, theme: UiTheme) {
    let focused = app.focus == Focus::Watchlist;
    let block = theme.panel_block(&t!("tui.panels.watchlist"), focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(search) = &app.stock_search {
        let rows = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(SEARCH_FIELD_HEIGHT),
        ])
        .split(inner);
        render_stock_search_results(frame, rows[0], search, theme);
        render_stock_search_field(frame, rows[1], search, theme);
        return;
    }

    if app.watchlist.is_empty() {
        frame.render_widget(
            Paragraph::new(t!("messages.no_data")).style(theme.dim()),
            inner,
        );
        return;
    }

    let width = inner.width as usize;
    let cols = watchlist_columns(width);

    let mut lines = vec![Line::from(
        watchlist_row_spans(
            "",
            &t!("headers.code"),
            &t!("headers.name"),
            &t!("headers.price"),
            &t!("headers.change_pct"),
            &cols,
            theme.section(),
            theme.section(),
            theme.section(),
        ),
    )];

    for (i, sym) in app.watchlist.iter().enumerate() {
        let q = app.quotes.get(&sym.stock_code);
        let name = q.map(|q| q.short_name.as_str()).unwrap_or("");
        let price = q.map(|q| format_price(q.price)).unwrap_or_else(|| "-".into());
        let chg = q
            .map(|q| format_change_pct(q.change_pct))
            .unwrap_or_else(|| "-".into());
        let value_style = q
            .map(|q| theme.change_style(q.change_pct))
            .unwrap_or(theme.text());
        let marker = if i == app.selected { "▶" } else { " " };
        let marker_style = if i == app.selected {
            theme.focus()
        } else {
            theme.text()
        };
        lines.push(Line::from(watchlist_row_spans(
            marker,
            &sym.stock_code,
            name,
            &price,
            &chg,
            &cols,
            marker_style,
            theme.text(),
            value_style,
        )));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn search_name_width(width: usize) -> usize {
    width
        .saturating_sub(WATCHLIST_MARKER_W + SEARCH_CODE_W + SEARCH_MARKET_W + WATCHLIST_GAP * 3)
        .max(8)
}

fn render_stock_search_results(frame: &mut Frame, area: Rect, search: &StockSearch, theme: UiTheme) {
    let width = area.width as usize;
    let name_w = search_name_width(width);
    let mut lines = vec![Line::from(vec![
        Span::styled(pad_display_end("", WATCHLIST_MARKER_W), theme.section()),
        Span::raw(" ".repeat(WATCHLIST_GAP)),
        Span::styled(pad_display_end(&t!("headers.code"), SEARCH_CODE_W), theme.section()),
        Span::raw(" ".repeat(WATCHLIST_GAP)),
        Span::styled(pad_display_end(&t!("headers.name"), name_w), theme.section()),
        Span::raw(" ".repeat(WATCHLIST_GAP)),
        Span::styled(pad_display_end(&t!("headers.market"), SEARCH_MARKET_W), theme.section()),
    ])];

    if search.input.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            t!("tui.search.empty").to_string(),
            theme.dim(),
        )));
    } else if search.loading && search.results.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("tui.search.loading").to_string(),
            theme.dim(),
        )));
    } else if search.results.is_empty() {
        lines.push(Line::from(Span::styled(
            t!("messages.no_data").to_string(),
            theme.dim(),
        )));
    } else {
        for (i, hit) in search.results.iter().enumerate() {
            let marker = if i == search.selected { "▶" } else { " " };
            let marker_style = if i == search.selected {
                theme.highlight()
            } else {
                theme.text()
            };
            lines.push(Line::from(vec![
                Span::styled(pad_display_end(marker, WATCHLIST_MARKER_W), marker_style),
                Span::raw(" ".repeat(WATCHLIST_GAP)),
                Span::styled(
                    pad_display_end(&hit.stock_code, SEARCH_CODE_W),
                    if i == search.selected {
                        theme.highlight()
                    } else {
                        theme.text()
                    },
                ),
                Span::raw(" ".repeat(WATCHLIST_GAP)),
                Span::styled(
                    pad_display_end(&hit.short_name, name_w),
                    if i == search.selected {
                        theme.highlight()
                    } else {
                        theme.text()
                    },
                ),
                Span::raw(" ".repeat(WATCHLIST_GAP)),
                Span::styled(
                    pad_display_end(&hit.market, SEARCH_MARKET_W),
                    if i == search.selected {
                        theme.highlight()
                    } else {
                        theme.dim()
                    },
                ),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        area,
    );
}

fn render_stock_search_field(frame: &mut Frame, area: Rect, search: &StockSearch, theme: UiTheme) {
    let title = format!(" {} ", t!("tui.search.title"));
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(theme.search_border_editing())
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let display = format!("/ {}{}", search.input, "▌");
    frame.render_widget(
        Paragraph::new(display).style(theme.text()),
        inner,
    );
}

fn render_quote(frame: &mut Frame, area: Rect, app: &App, theme: UiTheme) {
    let focused = app.focus == Focus::Quote;
    let block = theme.panel_block(&t!("tui.panels.quote"), focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(q) = app.quote.as_ref() else {
        frame.render_widget(
            Paragraph::new(empty_label(app.loading)).style(theme.dim()),
            inner,
        );
        return;
    };

    let rows_data = quote_rows(q, app.valuation.as_ref(), theme);
    let width = inner.width as usize;
    let grid_lines = quote_grid_lines(&rows_data, width, theme);
    let visible = inner.height as usize;
    let line_count = grid_lines.len();
    let max_scroll = line_count.saturating_sub(visible);
    let scroll = app.quote_scroll.min(max_scroll);

    let lines: Vec<Line> = grid_lines.into_iter().skip(scroll).take(visible).collect();
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );

    if line_count > visible {
        let mut state = ScrollbarState::new(line_count).position(scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            inner,
            &mut state,
        );
    }
}

fn render_kline(frame: &mut Frame, area: Rect, app: &App, theme: UiTheme) {
    let focused = app.focus == Focus::Kline;
    let title = match app.kline_view {
        KlineView::Timeline => {
            let period = kline::timeline_period_label(
                app.timeline_scope,
                app.timeline_day,
                &app.intraday,
            );
            format!("{} ({})", t!("tui.kline.timeline"), period)
        }
        _ => {
            let period = kline_label(app.kline_type).to_string();
            format!(
                "{} ({}) [{}]",
                t!("tui.panels.kline"),
                period,
                app.kline_view.label(),
            )
        }
    };
    let block = theme.panel_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let empty = match app.kline_view {
        KlineView::Timeline => app.visible_intraday().is_empty(),
        _ => app.kline.is_empty(),
    };
    if empty {
        frame.render_widget(
            Paragraph::new(empty_label(app.loading)).style(theme.dim()),
            inner,
        );
        return;
    }

    match app.kline_view {
        KlineView::Timeline => render_timeline_chart(frame, inner, app, theme),
        KlineView::Table => render_kline_table(frame, inner, app, theme),
        KlineView::Chart => render_kline_chart(frame, inner, app, theme),
    }
}

fn render_kline_table(frame: &mut Frame, inner: Rect, app: &App, theme: UiTheme) {
    let width = inner.width as usize;
    let date_w = 11;
    let num_w = 8;
    let vol_w = width.saturating_sub(date_w + num_w * 4 + 9 + 6);

    let header = Line::from(vec![
        Span::styled(pad_display_end(&t!("headers.date"), date_w), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.open"), num_w), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.high"), num_w), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.low"), num_w), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.close"), num_w), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.volume"), vol_w.max(8)), theme.section()),
        Span::raw("  "),
        Span::styled(pad_display_end(&t!("headers.change_pct"), 9), theme.section()),
    ]);

    let visible = inner.height.saturating_sub(1) as usize;
    let end = (app.kline_scroll + 1).min(app.kline.len());
    let start = end.saturating_sub(visible);

    let mut lines = vec![header];
    for b in &app.kline[start..end] {
        let chg_style = theme.change_style(b.change_pct);
        lines.push(Line::from(vec![
            Span::styled(format!("{}  ", b.trade_date), theme.dim()),
            Span::styled(format!("{}  ", pad_display_end(&format_price(b.open), num_w)), theme.text()),
            Span::styled(format!("{}  ", pad_display_end(&format_price(b.high), num_w)), theme.text()),
            Span::styled(format!("{}  ", pad_display_end(&format_price(b.low), num_w)), theme.text()),
            Span::styled(format!("{}  ", pad_display_end(&format_price(b.close), num_w)), theme.text()),
            Span::styled(format!("{}  ", format_volume_compact(b.volume)), theme.text()),
            Span::styled(format_change_pct(b.change_pct), chg_style),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );

    if app.kline.len() > visible {
        let mut state = ScrollbarState::new(app.kline.len()).position(app.kline_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            inner,
            &mut state,
        );
    }
}

fn render_timeline_chart(frame: &mut Frame, inner: Rect, app: &App, theme: UiTheme) {
    let visible = app.visible_intraday();
    let multi_day = app.timeline_scope == kline::TimelineScope::FiveDays;
    let quote_pre_close = app.quote.as_ref().and_then(|q| q.pre_close);
    let win = kline::timeline_window(
        &visible,
        app.kline_scroll,
        inner.width,
        multi_day,
        quote_pre_close,
    );
    if win.prices.is_empty() {
        frame.render_widget(
            Paragraph::new(t!("messages.no_data")).style(theme.dim()),
            inner,
        );
        return;
    }

    let [price_area, volume_area] = kline::split_chart_areas(inner);
    let (bull, bear, _) = theme.chart_colors();
    let axis_style = theme.dim();
    let bar_width = kline::TIMELINE_BAR_WIDTH;
    let bar_gap = kline::TIMELINE_BAR_GAP;

    let price_series = LineSeries::new(&win.prices).style(theme.text());
    let avg_series = LineSeries::new(&win.avg_prices).style(Style::new().fg(theme.chart_avg()));
    let mut price_chart = LineChart::new(price_series)
        .line(avg_series)
        .width(bar_width)
        .gap(bar_gap)
        .time_axis(TimeAxis::default().style(axis_style).labels(&win.labels));

    if let Some(pre_close) = win.pre_close {
        price_chart = price_chart.underlay(
            TrendLine::at(pre_close)
                .dashed()
                .style(axis_style)
                .autoscale(false),
        );
    }

    price_chart = match win.value_bounds {
        Some(bounds) => price_chart.value_axis(ValueAxis::default().style(axis_style).bounds(bounds)),
        None => price_chart.value_axis(ValueAxis::default().style(axis_style)),
    };

    let volume_series = VolumeSeries::new(&win.volumes)
        .width(bar_width)
        .gap(bar_gap)
        .bull_style(bull)
        .bear_style(bear);
    let volume_chart = VolumeChart::new(volume_series)
        .value_axis(ValueAxis::default().style(axis_style))
        .time_axis(TimeAxis::default().style(axis_style).labels(&win.labels));

    frame.render_widget(&price_chart, price_area);
    frame.render_widget(&volume_chart, volume_area);
}

fn render_kline_chart(frame: &mut Frame, inner: Rect, app: &App, theme: UiTheme) {
    let win = kline::window(
        &app.kline,
        app.kline_scroll,
        inner.width,
        app.kline_type,
    );
    if win.candles.is_empty() {
        frame.render_widget(
            Paragraph::new(t!("messages.no_data")).style(theme.dim()),
            inner,
        );
        return;
    }

    let [price_area, volume_area] = kline::split_chart_areas(inner);
    let (bull, bear, wick) = theme.chart_colors();
    let axis_style = theme.dim();

    let candle_series = CandleSeries::new(&win.candles)
        .width(1.0)
        .gap(1.0)
        .bull_style(bull)
        .bear_style(bear)
        .wick_style(wick);
    let price_chart = CandlestickChart::new(candle_series)
        .price_axis(PriceAxis::default().style(axis_style))
        .time_axis(TimeAxis::default().style(axis_style).labels(&win.labels));

    let volume_series = VolumeSeries::new(&win.volumes)
        .width(1.0)
        .gap(1.0)
        .bull_style(bull)
        .bear_style(bear);
    let volume_chart = VolumeChart::new(volume_series)
        .value_axis(ValueAxis::default().style(axis_style))
        .time_axis(TimeAxis::default().style(axis_style).labels(&win.labels));

    frame.render_widget(&price_chart, price_area);
    frame.render_widget(&volume_chart, volume_area);
}

fn render_feeds(frame: &mut Frame, area: Rect, app: &App, theme: UiTheme) {
    if app.feeds.is_empty() {
        return;
    }
    let count = app.feeds.len() as u32;
    let constraints: Vec<Constraint> = (0..app.feeds.len())
        .map(|_| Constraint::Ratio(1, count))
        .collect();
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    for (index, panel) in app.feeds.iter().enumerate() {
        let focused = matches!(app.focus, Focus::Feed(i) if i == index);
        render_feed_panel(frame, panels[index], panel, focused, theme);
    }
}

fn render_feed_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &super::feed::FeedPanel,
    focused: bool,
    theme: UiTheme,
) {
    let title = panel.page_title();
    let block = theme.panel_block(&title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match &panel.data {
        FeedData::Empty => {
            frame.render_widget(
                Paragraph::new(empty_label(panel.loading)).style(theme.dim()),
                inner,
            );
        }
        FeedData::Articles(articles) => {
            render_article_list(
                frame,
                inner,
                articles,
                panel.scroll,
                panel.selected,
                theme,
            );
        }
        FeedData::Reports(reports) => {
            render_report_list(frame, inner, reports, panel.scroll, panel.selected, theme);
        }
        FeedData::Institutional(rows) => {
            render_institutional_list(frame, inner, rows, panel.scroll, panel.selected, theme);
        }
        FeedData::TechnicalAnalysis(ta) => {
            render_ta_panel(frame, inner, ta, panel.selected, theme);
        }
        FeedData::Analyst(analyst) => {
            render_analyst_panel(frame, inner, analyst, panel.selected, theme);
        }
    }
}

fn render_article_list(
    frame: &mut Frame,
    inner: Rect,
    articles: &[tenk::NewsArticle],
    scroll: usize,
    selected: usize,
    theme: UiTheme,
) {
    let entries: Vec<ListEntry> = articles
        .iter()
        .map(|article| ListEntry {
            time: format_news_time_compact(article.publish_time),
            meta: &article.source,
            title: &article.title,
        })
        .collect();
    let lines = list_lines_compact(
        &entries,
        scroll,
        selected,
        inner.height as usize,
        inner.width as usize,
        theme,
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_report_list(
    frame: &mut Frame,
    inner: Rect,
    reports: &[tenk::ResearchReportData],
    scroll: usize,
    selected: usize,
    theme: UiTheme,
) {
    let entries: Vec<ListEntry> = reports
        .iter()
        .map(|report| ListEntry {
            time: format_report_time_compact(report.publish_date),
            meta: &report.institution,
            title: &report.title,
        })
        .collect();
    let lines = list_lines_compact(
        &entries,
        scroll,
        selected,
        inner.height as usize,
        inner.width as usize,
        theme,
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_institutional_list(
    frame: &mut Frame,
    inner: Rect,
    rows: &[tenk::InstitutionalResearchData],
    scroll: usize,
    selected: usize,
    theme: UiTheme,
) {
    struct RowView {
        time: String,
        meta: String,
        title: String,
    }
    let owned: Vec<RowView> = rows
        .iter()
        .map(|row| {
            let meta = if row.institution_count > 0 {
                format!("{} {}家", row.stock_code, row.institution_count)
            } else {
                row.stock_code.clone()
            };
            RowView {
                time: format_institutional_time(row.research_date),
                meta,
                title: row.stock_name.clone(),
            }
        })
        .collect();
    let entries: Vec<ListEntry> = owned
        .iter()
        .map(|row| ListEntry {
            time: row.time.clone(),
            meta: &row.meta,
            title: &row.title,
        })
        .collect();
    let lines = list_lines_compact(
        &entries,
        scroll,
        selected,
        inner.height as usize,
        inner.width as usize,
        theme,
    );
    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_ta_panel(
    frame: &mut Frame,
    inner: Rect,
    ta: &tenk::TvTechnicalAnalysis,
    scroll: usize,
    theme: UiTheme,
) {
    let mut periods: Vec<_> = ta.periods.iter().collect();
    periods.sort_by_key(|(name, _)| name.as_str());
    let mut lines: Vec<Line> = Vec::new();
    for (period, advice) in periods {
        lines.push(Line::from(format!(
            "{}: {}",
            period,
            advice_label(advice.overall)
        )));
    }
    if lines.is_empty() {
        lines.push(Line::from(t!("messages.no_data").to_string()));
    }
    let visible: Vec<Line> = lines.into_iter().skip(scroll).take(inner.height as usize).collect();
    frame.render_widget(
        Paragraph::new(Text::from(visible)).style(theme.text()),
        inner,
    );
}

fn render_analyst_panel(
    frame: &mut Frame,
    inner: Rect,
    analyst: &tenk::TvAnalystData,
    scroll: usize,
    theme: UiTheme,
) {
    let target = analyst
        .price_targets
        .average
        .map(format_price)
        .unwrap_or_else(|| "-".into());
    let lines = vec![
        Line::from(format!(
            "{}: {} / {} / {}",
            t!("tui.feeds.analyst_ratings"),
            analyst.ratings.buy,
            analyst.ratings.hold,
            analyst.ratings.sell
        )),
        Line::from(format!(
            "{}: {}",
            t!("tui.feeds.analyst_target"),
            target
        )),
    ];
    let visible: Vec<Line> = lines.into_iter().skip(scroll).take(inner.height as usize).collect();
    frame.render_widget(
        Paragraph::new(Text::from(visible)).style(theme.text()),
        inner,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App, theme: UiTheme) {
    if matches!(app.overlay, Overlay::Command(_)) {
        return;
    }

    let now = Local::now().format("%H:%M:%S").to_string();
    let sym = app.current_code().unwrap_or("-");
    let status_style = if app.status.contains("live") || app.status.contains("实时") {
        theme.status()
    } else {
        theme.dim()
    };
    let left = Line::from(vec![
        Span::styled(format!("{now} · {sym} · {} · ", app.source_label()), theme.dim()),
        Span::styled(app.status.clone(), status_style),
    ]);

    let hints = if app.dialog.is_some() {
        t!("tui.hints.dialog").to_string()
    } else if app.stock_search.is_some() {
        t!("tui.hints.search").to_string()
    } else if matches!(app.overlay, Overlay::Help) {
        t!("tui.hints.help").to_string()
    } else {
        focus_hints(app.focus)
    };

    let chunks = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(left).wrap(Wrap { trim: false }),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(hints)
            .style(theme.dim())
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn focus_hints(focus: Focus) -> String {
    match focus {
        Focus::Watchlist => t!("tui.hints.watchlist").to_string(),
        Focus::Quote => t!("tui.hints.quote").to_string(),
        Focus::Kline => t!("tui.hints.kline").to_string(),
        Focus::Feed(_) => t!("tui.hints.feed").to_string(),
    }
}

fn kline_label(k: tenk::KLineType) -> &'static str {
    match k {
        tenk::KLineType::Daily => "D",
        tenk::KLineType::Weekly => "W",
        tenk::KLineType::Monthly => "M",
        tenk::KLineType::Quarterly => "Q",
        tenk::KLineType::Min5 => "5m",
        tenk::KLineType::Min15 => "15m",
        tenk::KLineType::Min30 => "30m",
        tenk::KLineType::Min60 => "60m",
    }
}

fn empty_label(loading: bool) -> String {
    if loading {
        t!("tui.status.loading").to_string()
    } else {
        t!("messages.no_data").to_string()
    }
}
