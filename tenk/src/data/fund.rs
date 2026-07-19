//! ETF data structures.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::Exchange;

/// ETF basic information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFCode {
    /// Fund code
    pub fund_code: String,
    /// Short name
    pub short_name: String,
    /// Exchange
    pub exchange: Exchange,
    /// Net asset value
    pub net_value: Option<f64>,
}

impl ETFCode {
    /// Creates a new ETF code.
    pub fn new(fund_code: String, short_name: String, exchange: Exchange) -> Self {
        Self {
            fund_code,
            short_name,
            exchange,
            net_value: None,
        }
    }

    /// Returns the full symbol with exchange prefix.
    pub fn full_symbol(&self) -> String {
        format!("{}{}", self.exchange.market_prefix(), self.fund_code)
    }
}

/// ETF historical market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFMarketData {
    /// Fund code
    pub fund_code: String,
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
    /// Volume
    pub volume: u64,
    /// Amount
    pub amount: f64,
    /// Price change
    pub change: Option<f64>,
    /// Change percentage
    pub change_pct: Option<f64>,
}

/// Real-time ETF market data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFCurrentData {
    /// Fund code
    pub fund_code: String,
    /// Short name
    pub short_name: String,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: Option<f64>,
    /// Change percentage
    pub change_pct: Option<f64>,
    /// Volume
    pub volume: u64,
    /// Amount
    pub amount: f64,
    /// Open price
    pub open: Option<f64>,
    /// High price
    pub high: Option<f64>,
    /// Low price
    pub low: Option<f64>,
}

/// ETF intraday minute-level data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETFMinuteData {
    /// Fund code
    pub fund_code: String,
    /// Trade time
    pub trade_time: DateTime<Utc>,
    /// Current price
    pub price: f64,
    /// Price change
    pub change: f64,
    /// Change percentage
    pub change_pct: f64,
    /// Volume
    pub volume: u64,
    /// Average price
    pub avg_price: f64,
    /// Amount
    pub amount: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etf_code_full_symbol() {
        let etf = ETFCode::new("510300".to_string(), "沪深300ETF".to_string(), Exchange::SH);
        assert_eq!(etf.full_symbol(), "sh510300");

        let etf_sz = ETFCode::new("159915".to_string(), "创业板ETF".to_string(), Exchange::SZ);
        assert_eq!(etf_sz.full_symbol(), "sz159915");
    }
}
