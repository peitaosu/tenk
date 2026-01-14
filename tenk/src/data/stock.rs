//! Stock data structures.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    SH,
    SZ,
    BJ,
    Unknown,
}

impl Exchange {
    pub fn from_stock_code(code: &str) -> Self {
        match code.chars().next() {
            Some('6') => Exchange::SH,
            Some('0') | Some('3') => Exchange::SZ,
            Some('4') | Some('8') => Exchange::BJ,
            _ => Exchange::Unknown,
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KLineType {
    Daily = 1,
    Weekly = 2,
    Monthly = 3,
    Quarterly = 4,
    Min5 = 5,
    Min15 = 15,
    Min30 = 30,
    Min60 = 60,
}

impl KLineType {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AdjustType {
    None = 0,
    #[default]
    Forward = 1,
    Backward = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCode {
    pub stock_code: String,
    pub short_name: String,
    pub exchange: Exchange,
    pub list_date: Option<NaiveDate>,
}

impl StockCode {
    pub fn new(stock_code: String, short_name: String, exchange: Exchange) -> Self {
        Self {
            stock_code,
            short_name,
            exchange,
            list_date: None,
        }
    }

    pub fn full_symbol(&self) -> String {
        format!("{}{}", self.exchange.market_prefix(), self.stock_code)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub stock_code: String,
    pub trade_time: DateTime<Utc>,
    pub trade_date: NaiveDate,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u64,
    pub amount: f64,
    pub change: f64,
    pub change_pct: f64,
    pub turnover_ratio: f64,
    pub pre_close: f64,
}

impl MarketData {
    pub fn is_valid(&self) -> bool {
        self.volume > 0 && self.amount > 0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMarketData {
    pub stock_code: String,
    pub short_name: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub amount: f64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub pre_close: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinuteData {
    pub stock_code: String,
    pub trade_time: DateTime<Utc>,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: u64,
    pub avg_price: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub stock_code: String,
    pub short_name: String,
    pub sell_prices: [f64; 5],
    pub sell_volumes: [u64; 5],
    pub buy_prices: [f64; 5],
    pub buy_volumes: [u64; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    pub stock_code: String,
    pub trade_time: DateTime<Utc>,
    pub price: f64,
    pub volume: u64,
    pub direction: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub stock_code: String,
    pub full_name: String,
    pub short_name: String,
    pub exchange: Exchange,
    pub industry: Option<String>,
    pub total_shares: Option<u64>,
    pub circulating_shares: Option<u64>,
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
