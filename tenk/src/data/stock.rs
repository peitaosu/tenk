//! Stock data structures.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Stock exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    /// Shanghai Stock Exchange
    SH,
    /// Shenzhen Stock Exchange
    SZ,
    /// Beijing Stock Exchange
    BJ,
    /// Unknown exchange
    Unknown,
}

impl Exchange {
    /// Determines exchange from stock code prefix.
    pub fn from_stock_code(code: &str) -> Self {
        let first_two: String = code.chars().take(2).collect();
        match first_two.as_str() {
            // Shanghai: 6xxxxx stocks, 11xxxx bonds, 5xxxxx ETFs
            s if s.starts_with('6') => Exchange::SH,
            "11" => Exchange::SH,
            s if s.starts_with('5') => Exchange::SH,
            // Shenzhen: 0xxxxx/3xxxxx stocks, 12xxxx bonds, 1xxxxx ETFs
            s if s.starts_with('0') || s.starts_with('3') => Exchange::SZ,
            "12" => Exchange::SZ,
            s if s.starts_with('1') && !s.starts_with("11") => Exchange::SZ,
            // Beijing: 4xxxxx/8xxxxx
            s if s.starts_with('4') || s.starts_with('8') => Exchange::BJ,
            _ => Exchange::Unknown,
        }
    }

    /// Returns the market prefix string.
    pub fn market_prefix(&self) -> &'static str {
        match self {
            Exchange::SH => "sh",
            Exchange::SZ => "sz",
            Exchange::BJ => "bj",
            Exchange::Unknown => "",
        }
    }
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::SH => write!(f, "SH"),
            Exchange::SZ => write!(f, "SZ"),
            Exchange::BJ => write!(f, "BJ"),
            Exchange::Unknown => write!(f, "Unknown"),
        }
    }
}

/// K-line (candlestick) time period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KLineType {
    /// Daily K-line
    Daily = 1,
    /// Weekly K-line
    Weekly = 2,
    /// Monthly K-line
    Monthly = 3,
    /// Quarterly K-line
    Quarterly = 4,
    /// 5-minute K-line
    Min5 = 5,
    /// 15-minute K-line
    Min15 = 15,
    /// 30-minute K-line
    Min30 = 30,
    /// 60-minute K-line
    Min60 = 60,
}

impl KLineType {
    /// Converts to API value code.
    pub fn to_api_value(&self) -> u32 {
        match self {
            KLineType::Daily => 101,
            KLineType::Weekly => 102,
            KLineType::Monthly => 103,
            KLineType::Quarterly => 104,
            KLineType::Min5 => 5,
            KLineType::Min15 => 15,
            KLineType::Min30 => 30,
            KLineType::Min60 => 60,
        }
    }
}

/// Price adjustment type for historical data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AdjustType {
    /// No adjustment (raw price)
    None = 0,
    /// Forward adjusted (前复权)
    #[default]
    Forward = 1,
    /// Backward adjusted (后复权)
    Backward = 2,
}

/// Stock basic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCode {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Exchange
    pub exchange: Exchange,
    /// Listing date
    pub list_date: Option<NaiveDate>,
}

impl StockCode {
    /// Creates a new stock code.
    pub fn new(stock_code: String, short_name: String, exchange: Exchange) -> Self {
        Self {
            stock_code,
            short_name,
            exchange,
            list_date: None,
        }
    }

    /// Returns the full symbol with exchange prefix.
    pub fn full_symbol(&self) -> String {
        format!("{}{}", self.exchange.market_prefix(), self.stock_code)
    }
}

/// Historical market data (K-line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Trade date
    pub trade_date: NaiveDate,
    /// Open price
    pub open: f64,
    /// Close price
    pub close: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Volume (shares)
    pub volume: u64,
    /// Amount (currency)
    pub amount: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Turnover ratio
    pub turnover_ratio: f64,
    /// Previous close
    pub pre_close: f64,
}

impl MarketData {
    /// Returns true if data is valid.
    pub fn is_valid(&self) -> bool {
        self.volume > 0 && self.amount > 0.0
    }
}

/// Real-time stock market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMarketData {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume (shares)
    pub volume: u64,
    /// Amount (currency)
    pub amount: f64,
    /// Open price
    pub open: Option<f64>,
    /// High price
    pub high: Option<f64>,
    /// Low price
    pub low: Option<f64>,
    /// Previous close
    pub pre_close: Option<f64>,
}

/// Intraday minute-level data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinuteData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume (shares)
    pub volume: u64,
    /// Average price
    pub avg_price: f64,
    /// Amount (currency)
    pub amount: f64,
}

/// Level 1 order book data (5 levels).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    /// Stock code
    pub stock_code: String,
    /// Short name
    pub short_name: String,
    /// Sell prices (5 levels)
    pub sell_prices: [f64; 5],
    /// Sell volumes (5 levels)
    pub sell_volumes: [u64; 5],
    /// Buy prices (5 levels)
    pub buy_prices: [f64; 5],
    /// Buy volumes (5 levels)
    pub buy_volumes: [u64; 5],
}

/// Tick-by-tick transaction data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    /// Stock code
    pub stock_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Trade price
    pub price: f64,
    /// Trade volume
    pub volume: u64,
    /// Trade direction (B/S)
    pub direction: char,
}

/// Detailed stock information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    /// Stock code
    pub stock_code: String,
    /// Full company name
    pub full_name: String,
    /// Short name
    pub short_name: String,
    /// Exchange
    pub exchange: Exchange,
    /// Industry sector
    pub industry: Option<String>,
    /// Total shares
    pub total_shares: Option<u64>,
    /// Circulating shares
    pub circulating_shares: Option<u64>,
    /// Listing date
    pub list_date: Option<NaiveDate>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_from_code() {
        assert_eq!(Exchange::from_stock_code("600000"), Exchange::SH);
        assert_eq!(Exchange::from_stock_code("000001"), Exchange::SZ);
        assert_eq!(Exchange::from_stock_code("300001"), Exchange::SZ);
        assert_eq!(Exchange::from_stock_code("830001"), Exchange::BJ);
    }

    #[test]
    fn test_stock_code_full_symbol() {
        let stock = StockCode::new("600519".to_string(), "贵州茅台".to_string(), Exchange::SH);
        assert_eq!(stock.full_symbol(), "sh600519");
    }

    #[test]
    fn test_kline_type_api_value() {
        assert_eq!(KLineType::Daily.to_api_value(), 101);
        assert_eq!(KLineType::Min5.to_api_value(), 5);
    }
}
