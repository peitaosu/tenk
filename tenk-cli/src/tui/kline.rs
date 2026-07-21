use chrono::{DateTime, NaiveDate, Utc};
use chandelier::{Candle, Volume};
use ratatui::layout::Rect;
use rust_i18n::t;
use tenk::{format_cn_market_time, cn_market_date, KLineType, MarketData, MinuteData};

pub const TIMELINE_BAR_WIDTH: f64 = 1.0;
pub const TIMELINE_BAR_GAP: f64 = 0.0;
const TIMELINE_MIN_SPAN_FRAC: f64 = 0.02;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineScope {
    #[default]
    Day,
    FiveDays,
}

pub fn intraday_dates(data: &[MinuteData]) -> Vec<NaiveDate> {
    let mut dates: Vec<NaiveDate> = data.iter().map(|m| cn_market_date(m.trade_time)).collect();
    dates.sort();
    dates.dedup();
    dates.reverse();
    dates
}

pub fn filter_intraday(data: &[MinuteData], scope: TimelineScope, day_index: usize) -> Vec<MinuteData> {
    match scope {
        TimelineScope::FiveDays => data.to_vec(),
        TimelineScope::Day => {
            let dates = intraday_dates(data);
            let Some(day) = dates.get(day_index) else {
                return Vec::new();
            };
            data.iter()
                .filter(|m| cn_market_date(m.trade_time) == *day)
                .cloned()
                .collect()
        }
    }
}

pub fn timeline_period_label(scope: TimelineScope, day_index: usize, data: &[MinuteData]) -> String {
    match scope {
        TimelineScope::FiveDays => t!("tui.kline.five_days").to_string(),
        TimelineScope::Day => intraday_dates(data)
            .get(day_index)
            .map(|d| d.format("%m-%d").to_string())
            .unwrap_or_else(|| t!("tui.kline.timeline").to_string()),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KlineView {
    Timeline,
    Chart,
    Table,
}

impl Default for KlineView {
    fn default() -> Self {
        Self::Chart
    }
}

impl KlineView {
    pub fn toggle(self) -> Self {
        match self {
            Self::Timeline => Self::Chart,
            Self::Chart => Self::Table,
            Self::Table => Self::Timeline,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Timeline => t!("tui.kline.timeline").to_string(),
            Self::Chart => t!("tui.kline.chart").to_string(),
            Self::Table => t!("tui.kline.table").to_string(),
        }
    }
}

pub struct KlineWindow {
    pub candles: Vec<Candle>,
    pub volumes: Vec<Volume>,
    pub labels: Vec<String>,
}

pub struct TimelineWindow {
    pub prices: Vec<Option<f64>>,
    pub avg_prices: Vec<Option<f64>>,
    pub volumes: Vec<Volume>,
    pub labels: Vec<String>,
    pub pre_close: Option<f64>,
    pub value_bounds: Option<[f64; 2]>,
}

struct TimelineBar {
    price: Option<f64>,
    avg_price: Option<f64>,
    volume: Volume,
    trade_time: Option<DateTime<Utc>>,
    session_start: bool,
}

pub fn timeline_pre_close(quote_pre_close: Option<f64>, mins: &[MinuteData]) -> Option<f64> {
    if let Some(pre) = quote_pre_close.filter(|p| *p > 0.0) {
        return Some(pre);
    }
    mins.first().and_then(|m| {
        if m.change.abs() > f64::EPSILON {
            Some(m.price - m.change)
        } else {
            None
        }
    })
}

pub fn timeline_value_bounds(pre_close: f64, prices: &[Option<f64>]) -> [f64; 2] {
    let max_dev = prices
        .iter()
        .filter_map(|p| *p)
        .map(|p| (p - pre_close).abs())
        .fold(0.0, f64::max);
    let half = max_dev.max(pre_close * TIMELINE_MIN_SPAN_FRAC / 2.0);
    [pre_close - half, pre_close + half]
}

pub fn window(bars: &[MarketData], scroll_end: usize, width: u16, kline_type: KLineType) -> KlineWindow {
    let cap = ((width.saturating_sub(10) / 2).max(8)) as usize;
    let end = (scroll_end + 1).min(bars.len());
    let start = end.saturating_sub(cap);
    let slice = &bars[start..end];
    KlineWindow {
        candles: slice
            .iter()
            .map(|b| Candle::new(b.open, b.high, b.low, b.close))
            .collect(),
        volumes: slice
            .iter()
            .map(|b| {
                Volume::new(b.volume as f64).with_direction(
                    Candle::new(b.open, b.high, b.low, b.close).direction(),
                )
            })
            .collect(),
        labels: slice
            .iter()
            .map(|b| format_axis_label(b.trade_time, b.trade_date.to_string(), kline_type))
            .collect(),
    }
}

pub fn timeline_window(
    mins: &[MinuteData],
    scroll_end: usize,
    width: u16,
    multi_day: bool,
    quote_pre_close: Option<f64>,
) -> TimelineWindow {
    let bars = build_timeline_bars(mins, multi_day);
    if bars.is_empty() {
        return TimelineWindow {
            prices: vec![],
            avg_prices: vec![],
            volumes: vec![],
            labels: vec![],
            pre_close: None,
            value_bounds: None,
        };
    }

    let pre_close = timeline_pre_close(quote_pre_close, mins);
    let all_prices: Vec<Option<f64>> = bars.iter().map(|b| b.price).collect();
    let value_bounds = pre_close.map(|pre| timeline_value_bounds(pre, &all_prices));

    let cap = timeline_capacity(width);
    let end = (scroll_end + 1).min(bars.len());
    let start = end.saturating_sub(cap);
    let slice = &bars[start..end];
    let total = slice.len();

    TimelineWindow {
        prices: slice.iter().map(|b| b.price).collect(),
        avg_prices: slice.iter().map(|b| b.avg_price).collect(),
        volumes: slice.iter().map(|b| b.volume).collect(),
        labels: slice
            .iter()
            .enumerate()
            .map(|(i, b)| timeline_axis_label(i, total, b.trade_time, b.session_start, multi_day))
            .collect(),
        pre_close,
        value_bounds,
    }
}

pub fn split_chart_areas(area: Rect) -> [Rect; 2] {
    ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Percentage(67),
        ratatui::layout::Constraint::Percentage(33),
    ])
    .areas(area)
}

fn timeline_capacity(width: u16) -> usize {
    width.saturating_sub(10).max(20) as usize
}

fn build_timeline_bars(mins: &[MinuteData], multi_day: bool) -> Vec<TimelineBar> {
    let mut bars = Vec::with_capacity(mins.len());
    let mut prev_price = mins.first().map(|m| m.price);
    let mut prev_day: Option<NaiveDate> = None;

    for m in mins {
        let day = cn_market_date(m.trade_time);
        let session_start = prev_day != Some(day);
        if multi_day && prev_day.is_some() && session_start {
            bars.push(TimelineBar {
                price: None,
                avg_price: None,
                volume: Volume::new(0.0),
                trade_time: None,
                session_start: false,
            });
        }
        prev_day = Some(day);

        let open = prev_price.unwrap_or(m.price);
        let close = m.price;
        prev_price = Some(close);
        bars.push(TimelineBar {
            price: Some(m.price),
            avg_price: Some(m.avg_price),
            volume: Volume::new(m.volume as f64).with_direction(
                Candle::new(open, open.max(close), open.min(close), close).direction(),
            ),
            trade_time: Some(m.trade_time),
            session_start,
        });
    }

    bars
}

fn timeline_axis_label(
    index: usize,
    total: usize,
    trade_time: Option<DateTime<Utc>>,
    session_start: bool,
    multi_day: bool,
) -> String {
    let Some(trade_time) = trade_time else {
        return String::new();
    };
    if multi_day && session_start {
        return format_cn_market_time(trade_time, "%m-%d");
    }
    if total <= 1 {
        return format_minute_label(trade_time);
    }
    if index == 0 || index + 1 == total || index == total / 2 {
        return format_minute_label(trade_time);
    }
    String::new()
}

fn format_axis_label(trade_time: DateTime<Utc>, date: String, kline_type: KLineType) -> String {
    match kline_type {
        KLineType::Min5 | KLineType::Min15 | KLineType::Min30 | KLineType::Min60 => {
            format_cn_market_time(trade_time, "%m-%d %H:%M")
        }
        _ => date,
    }
}

fn format_minute_label(trade_time: DateTime<Utc>) -> String {
    format_cn_market_time(trade_time, "%H:%M")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn minute_at(hour: u32, minute: u32) -> MinuteData {
        MinuteData {
            stock_code: "600519".into(),
            trade_time: Utc.with_ymd_and_hms(2026, 7, 21, hour, minute, 0).unwrap(),
            price: 100.0,
            change: 1.0,
            change_pct: 1.0,
            volume: 1000,
            avg_price: 99.5,
            amount: 100_000.0,
        }
    }

    fn minute_on(day: u32, hour: u32, minute: u32, price: f64) -> MinuteData {
        MinuteData {
            stock_code: "600519".into(),
            trade_time: Utc.with_ymd_and_hms(2026, 7, day, hour, minute, 0).unwrap(),
            price,
            change: price - 100.0,
            change_pct: (price - 100.0) / 100.0 * 100.0,
            volume: 1000,
            avg_price: price - 0.5,
            amount: price * 1000.0,
        }
    }

    #[test]
    fn timeline_value_bounds_are_symmetric_around_pre_close() {
        let prices = vec![Some(98.0), Some(102.0), Some(100.0)];
        let bounds = timeline_value_bounds(100.0, &prices);
        assert!((bounds[0] - 98.0).abs() < f64::EPSILON);
        assert!((bounds[1] - 102.0).abs() < f64::EPSILON);
        assert!((bounds[0] + bounds[1] - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn timeline_value_bounds_use_minimum_span() {
        let prices = vec![Some(100.1), Some(99.9)];
        let bounds = timeline_value_bounds(100.0, &prices);
        assert!((bounds[1] - bounds[0] - 100.0 * TIMELINE_MIN_SPAN_FRAC).abs() < f64::EPSILON);
    }

    #[test]
    fn multi_day_timeline_inserts_session_gaps() {
        let mins = vec![
            minute_on(18, 9, 30, 100.0),
            minute_on(18, 9, 31, 100.5),
            minute_on(21, 9, 30, 101.0),
        ];
        let win = timeline_window(&mins, 10, 80, true, Some(100.0));
        assert!(win.prices.windows(2).any(|w| w[0].is_some() && w[1].is_none()));
        assert!(win.prices.iter().any(|p| p.is_none()));
    }

    #[test]
    fn timeline_window_scrolls_unified_for_single_day() {
        let mins: Vec<_> = (0..60)
            .map(|i| minute_at(10 + i / 30, i % 30))
            .collect();
        let full = timeline_window(&mins, mins.len(), 80, false, Some(100.0));
        let scrolled = timeline_window(&mins, 20, 30, false, Some(100.0));
        assert_eq!(full.prices.len(), mins.len());
        assert!(scrolled.prices.len() < full.prices.len());
    }

    #[test]
    fn session_start_labels_use_date_in_multi_day_mode() {
        let mins = vec![minute_on(21, 9, 30, 101.0)];
        let win = timeline_window(&mins, 0, 80, true, Some(100.0));
        assert_eq!(win.labels[0], "07-21");
    }
}
