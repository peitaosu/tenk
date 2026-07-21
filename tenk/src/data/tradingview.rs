use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::data::MarketData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvAdvice {
    StrongSell,
    Sell,
    Neutral,
    Buy,
    StrongBuy,
}

impl TvAdvice {
    pub fn from_score(score: f64) -> Self {
        if score <= -0.5 {
            Self::StrongSell
        } else if score <= -0.1 {
            Self::Sell
        } else if score < 0.1 {
            Self::Neutral
        } else if score < 0.5 {
            Self::Buy
        } else {
            Self::StrongBuy
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvTimeFrame {
    Min1,
    Min3,
    Min5,
    Min15,
    Min30,
    Min45,
    Min60,
    Min120,
    Min180,
    Min240,
    Daily,
    Weekly,
    Monthly,
}

impl TvTimeFrame {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::Min1 => "1",
            Self::Min3 => "3",
            Self::Min5 => "5",
            Self::Min15 => "15",
            Self::Min30 => "30",
            Self::Min45 => "45",
            Self::Min60 => "60",
            Self::Min120 => "120",
            Self::Min180 => "180",
            Self::Min240 => "240",
            Self::Daily => "1D",
            Self::Weekly => "1W",
            Self::Monthly => "1M",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "1" | "1m" | "min1" => Some(Self::Min1),
            "3" | "3m" | "min3" => Some(Self::Min3),
            "5" | "5m" | "min5" => Some(Self::Min5),
            "15" | "15m" | "min15" => Some(Self::Min15),
            "30" | "30m" | "min30" => Some(Self::Min30),
            "45" | "45m" | "min45" => Some(Self::Min45),
            "60" | "1h" | "min60" => Some(Self::Min60),
            "120" | "2h" | "min120" => Some(Self::Min120),
            "180" | "3h" | "min180" => Some(Self::Min180),
            "240" | "4h" | "min240" => Some(Self::Min240),
            "d" | "1d" | "daily" => Some(Self::Daily),
            "w" | "1w" | "weekly" => Some(Self::Weekly),
            "m" | "1mth" | "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvAssetFilter {
    Stock,
    Futures,
    Forex,
    Cfd,
    Crypto,
    Index,
    Economic,
    Bond,
    Funds,
}

impl TvAssetFilter {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::Stock => "stock",
            Self::Futures => "futures",
            Self::Forex => "forex",
            Self::Cfd => "cfd",
            Self::Crypto => "crypto",
            Self::Index => "index",
            Self::Economic => "economic",
            Self::Bond => "bond",
            Self::Funds => "funds",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "stock" | "stocks" => Some(Self::Stock),
            "futures" | "future" => Some(Self::Futures),
            "forex" | "fx" => Some(Self::Forex),
            "cfd" => Some(Self::Cfd),
            "crypto" | "cryptocurrency" => Some(Self::Crypto),
            "index" | "indices" => Some(Self::Index),
            "economic" | "economy" => Some(Self::Economic),
            "bond" | "bonds" => Some(Self::Bond),
            "funds" | "fund" | "etf" => Some(Self::Funds),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvChartType {
    HeikinAshi,
    Renko,
    LineBreak,
    Kagi,
    PointAndFigure,
    Range,
}

impl TvChartType {
    pub fn study_id(self) -> &'static str {
        match self {
            Self::HeikinAshi => "BarSetHeikenAshi@tv-basicstudies-60!",
            Self::Renko => "BarSetRenko@tv-prostudies-40!",
            Self::LineBreak => "BarSetPriceBreak@tv-prostudies-34!",
            Self::Kagi => "BarSetKagi@tv-prostudies-34!",
            Self::PointAndFigure => "BarSetPnF@tv-prostudies-34!",
            Self::Range => "BarSetRange@tv-basicstudies-72!",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "heikinashi" | "heikin_ashi" | "ha" => Some(Self::HeikinAshi),
            "renko" => Some(Self::Renko),
            "linebreak" | "line_break" => Some(Self::LineBreak),
            "kagi" => Some(Self::Kagi),
            "pointandfigure" | "pnf" => Some(Self::PointAndFigure),
            "range" => Some(Self::Range),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TvHotlistKind {
    Gainers,
    Losers,
    Active,
    PreMarketGainers,
    AfterHoursGainers,
}

impl TvHotlistKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "gainers" | "top_gainers" => Some(Self::Gainers),
            "losers" | "top_losers" => Some(Self::Losers),
            "active" | "most_active" | "volume" => Some(Self::Active),
            "premarket_gainers" | "pre_market_gainers" => Some(Self::PreMarketGainers),
            "afterhours_gainers" | "after_hours_gainers" => Some(Self::AfterHoursGainers),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvSymbolMatch {
    pub id: String,
    pub exchange: String,
    pub full_exchange: String,
    pub symbol: String,
    pub description: String,
    pub asset_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvPeriodAdvice {
    pub oscillators: TvAdvice,
    pub moving_averages: TvAdvice,
    pub overall: TvAdvice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvTechnicalAnalysis {
    pub symbol: String,
    pub periods: HashMap<String, TvPeriodAdvice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvIndicatorMeta {
    pub id: String,
    pub version: String,
    pub name: String,
    pub author_id: String,
    pub author_name: String,
    pub image: String,
    pub access: String,
    pub indicator_type: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvIndicatorInput {
    pub id: String,
    pub name: String,
    pub input_type: String,
    pub value: serde_json::Value,
    pub options: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvIndicatorSpec {
    pub pine_id: String,
    pub pine_version: String,
    pub description: String,
    pub short_description: String,
    pub inputs: Vec<TvIndicatorInput>,
    pub plots: HashMap<String, String>,
    pub script: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvUserSession {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub session: String,
    pub signature: String,
    pub auth_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvPinePermUser {
    pub id: i64,
    pub username: String,
    pub expiration: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvQuote {
    pub symbol: String,
    pub last_price: Option<f64>,
    pub change: Option<f64>,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub prev_close: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub description: Option<String>,
    pub exchange: Option<String>,
    pub currency: Option<String>,
    pub market_cap: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub sector: Option<String>,
    pub raw: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvChartOptions {
    pub timeframe: TvTimeFrame,
    pub range: usize,
    pub to: Option<i64>,
    pub adjustment: Option<String>,
    pub session: Option<String>,
    pub currency: Option<String>,
    pub chart_type: Option<TvChartType>,
    pub replay_from: Option<i64>,
}

impl Default for TvChartOptions {
    fn default() -> Self {
        Self {
            timeframe: TvTimeFrame::Daily,
            range: 100,
            to: None,
            adjustment: None,
            session: None,
            currency: None,
            chart_type: None,
            replay_from: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvChartBar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl TvChartBar {
    pub fn to_market_data(&self, symbol: &str) -> MarketData {
        use chrono::{NaiveDate, TimeZone, Utc};
        let seconds = if self.time > 1_000_000_000_000 {
            self.time / 1000
        } else {
            self.time
        };
        let dt = Utc.timestamp_opt(seconds, 0).single();
        let trade_date = dt
            .map(|d| d.date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default());
        let change = self.close - self.open;
        let change_pct = if self.open > 0.0 {
            change / self.open * 100.0
        } else {
            0.0
        };
        MarketData {
            stock_code: symbol.to_string(),
            trade_time: dt.unwrap_or_else(Utc::now),
            trade_date,
            open: self.open,
            close: self.close,
            high: self.high,
            low: self.low,
            volume: self.volume.round() as u64,
            amount: 0.0,
            change,
            change_pct,
            turnover_ratio: 0.0,
            pre_close: self.open,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMarketInfo {
    pub full_name: String,
    pub description: String,
    pub exchange: String,
    pub currency: String,
    pub asset_type: String,
    pub timezone: String,
    pub has_intraday: bool,
    pub is_replayable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvIndicatorPoint {
    pub time: i64,
    pub values: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvIndicatorSeries {
    pub symbol: String,
    pub indicator: String,
    pub points: Vec<TvIndicatorPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvTradeReport {
    pub entry_type: String,
    pub entry_price: f64,
    pub entry_time: i64,
    pub exit_price: f64,
    pub exit_time: i64,
    pub quantity: f64,
    pub profit: f64,
    pub cumulative: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvStrategyPerformance {
    pub net_profit: Option<f64>,
    pub net_profit_percent: Option<f64>,
    pub gross_profit: Option<f64>,
    pub gross_loss: Option<f64>,
    pub total_trades: Option<i64>,
    pub winning_trades: Option<i64>,
    pub losing_trades: Option<i64>,
    pub percent_profitable: Option<f64>,
    pub profit_factor: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub max_drawdown_percent: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvStrategyReport {
    pub symbol: String,
    pub indicator: String,
    pub currency: Option<String>,
    pub trades: Vec<TvTradeReport>,
    pub performance: TvStrategyPerformance,
    pub equity: Vec<f64>,
    pub drawdown: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvDrawingPoint {
    pub time: i64,
    pub price: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvDrawing {
    pub id: String,
    pub symbol: String,
    pub drawing_type: String,
    pub points: Vec<TvDrawingPoint>,
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvScreenerFilter {
    pub field: String,
    pub operation: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvScreenerRequest {
    pub market: String,
    pub columns: Vec<String>,
    pub filters: Vec<TvScreenerFilter>,
    pub sort_by: String,
    pub sort_order: String,
    pub range_start: usize,
    pub range_end: usize,
    pub preset: String,
}

impl Default for TvScreenerRequest {
    fn default() -> Self {
        Self {
            market: "global".to_string(),
            columns: vec![
                "name".into(),
                "close".into(),
                "change".into(),
                "volume".into(),
                "Recommend.All".into(),
            ],
            filters: Vec::new(),
            sort_by: "name".into(),
            sort_order: "asc".into(),
            range_start: 0,
            range_end: 50,
            preset: "all_stocks".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvScreenerRow {
    pub symbol: String,
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvScreenerResult {
    pub market: String,
    pub total_count: i64,
    pub rows: Vec<TvScreenerRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvCalendarEvent {
    pub id: String,
    pub title: String,
    pub country: String,
    pub indicator: String,
    pub period: String,
    pub source: String,
    pub actual: Option<f64>,
    pub previous: Option<f64>,
    pub forecast: Option<f64>,
    pub currency: String,
    pub unit: Option<String>,
    pub importance: i64,
    pub date: String,
    pub ticker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvReplayResult {
    pub symbol: String,
    pub bars: Vec<TvChartBar>,
    pub replay_end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvAnalystRatings {
    pub buy: u32,
    pub sell: u32,
    pub hold: u32,
    pub over: u32,
    pub under: u32,
    pub total: u32,
    pub mark: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvAnalystPriceTargets {
    pub average: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub median: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvAnalystForecasts {
    pub eps_next_fy: Option<f64>,
    pub revenue_next_fy: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvEstimatePoint {
    pub period: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TvEstimateSeries {
    pub points: Vec<TvEstimatePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TvAnalystEstimates {
    pub earnings_fq: TvEstimateSeries,
    pub revenue_fq: TvEstimateSeries,
    pub eps_forecast_fq: TvEstimateSeries,
    pub eps_actual_fq: TvEstimateSeries,
    pub earnings_fy: TvEstimateSeries,
    pub revenue_fy: TvEstimateSeries,
    pub eps_forecast_fy: TvEstimateSeries,
    pub eps_actual_fy: TvEstimateSeries,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvAnalystData {
    pub symbol: String,
    pub ratings: TvAnalystRatings,
    pub price_targets: TvAnalystPriceTargets,
    pub forecasts: TvAnalystForecasts,
    pub estimates: Option<TvAnalystEstimates>,
}
