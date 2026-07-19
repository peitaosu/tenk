//! East Money data source.

mod stock;
mod bond;
mod news;
mod extended;
mod market_extra;
mod derivatives;

use chrono::FixedOffset;
use serde::Deserialize;
use crate::error::DataResult;
use crate::request::{RequestConfig, RequestManager};

/// East Money data source.
#[derive(Debug, Clone)]
pub struct EastMoneySource {
    request: RequestManager,
}

impl EastMoneySource {
    pub fn new() -> DataResult<Self> {
        Self::try_new(None)
    }

    pub fn try_new(proxy: Option<&str>) -> DataResult<Self> {
        let config = RequestConfig::default().with_proxy_opt(proxy);
        Ok(Self::with_request_manager(RequestManager::new(config)?))
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        Self { request }
    }
}

impl Default for EastMoneySource {
    fn default() -> Self {
        Self::new().expect("Failed to create EastMoneySource")
    }
}

fn beijing_tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).unwrap()
}

#[derive(Debug, Deserialize)]
struct KLineResponse {
    /// K-line data
    data: Option<KLineData>,
}

#[derive(Debug, Deserialize)]
struct KLineData {
    /// K-line strings
    klines: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct MinuteResponse {
    /// Minute data
    data: Option<MinuteResponseData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinuteResponseData {
    /// Previous close price
    pre_close: f64,
    /// Trend data strings
    trends: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct StockListResponse {
    /// Stock list data
    data: Option<StockListData>,
}

#[derive(Debug, Deserialize)]
struct StockListData {
    /// Stock items
    diff: Option<Vec<StockItem>>,
}

#[derive(Debug, Deserialize)]
struct StockItem {
    /// Stock code
    #[serde(rename = "f12")]
    code: String,
    /// Stock name
    #[serde(rename = "f14")]
    name: String,
    /// Current price
    #[serde(rename = "f2", default)]
    price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct StockInfoResponse {
    /// Result data
    result: Option<StockInfoResult>,
}

#[derive(Debug, Deserialize)]
struct StockInfoResult {
    /// Stock info items
    data: Option<Vec<StockInfoItem>>,
}

#[derive(Debug, Deserialize)]
struct BondListResponse {
    /// Result data
    result: Option<BondListResult>,
}

#[derive(Debug, Deserialize)]
struct BondListResult {
    /// Bond items
    data: Option<Vec<BondItem>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BondItem {
    /// Bond code
    #[serde(rename = "SECURITY_CODE")]
    bond_code: String,
    /// Bond name
    #[serde(rename = "SECURITY_NAME_ABBR")]
    bond_name: String,
    /// Underlying stock code
    #[serde(rename = "CONVERT_STOCK_CODE", default)]
    stock_code: Option<String>,
    /// Short name
    #[serde(rename = "SECURITY_SHORT_NAME", default)]
    short_name: Option<String>,
    /// Subscription date
    #[serde(rename = "PUBLIC_START_DATE", default)]
    sub_date: Option<String>,
    /// Issue amount
    #[serde(rename = "ACTUAL_ISSUE_SCALE", default)]
    issue_amount: Option<f64>,
    /// Listing date
    #[serde(rename = "LISTING_DATE", default)]
    listing_date: Option<String>,
    /// Expiration date
    #[serde(rename = "EXPIRE_DATE", default)]
    expire_date: Option<String>,
    /// Conversion price
    #[serde(rename = "TRANSFER_PRICE", default)]
    convert_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteResponse {
    /// Quote data
    data: Option<BondQuoteData>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteData {
    /// Quote items
    diff: Option<Vec<BondQuoteItem>>,
}

#[derive(Debug, Deserialize)]
struct BondQuoteItem {
    /// Bond code
    #[serde(rename = "f12")]
    bond_code: String,
    /// Bond name
    #[serde(rename = "f14")]
    bond_name: String,
    /// Current price
    #[serde(rename = "f2", default)]
    price: Option<f64>,
    /// Change percentage
    #[serde(rename = "f3", default)]
    change_pct: Option<f64>,
    /// Price change
    #[serde(rename = "f4", default)]
    change: Option<f64>,
    /// Volume
    #[serde(rename = "f5", default)]
    volume: Option<u64>,
    /// Amount
    #[serde(rename = "f6", default)]
    amount: Option<f64>,
    /// High price
    #[serde(rename = "f15", default)]
    high: Option<f64>,
    /// Low price
    #[serde(rename = "f16", default)]
    low: Option<f64>,
    /// Open price
    #[serde(rename = "f17", default)]
    open: Option<f64>,
    /// Previous close
    #[serde(rename = "f18", default)]
    pre_close: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NewsResponse {
    /// Response code
    #[serde(default)]
    rc: i32,
    /// News items
    news: Option<Vec<NewsItem>>,
}

#[derive(Debug, Deserialize)]
struct NewsItem {
    /// News ID
    id: String,
    /// Title
    title: String,
    /// Digest/summary
    #[serde(default)]
    digest: String,
    /// Web URL
    #[serde(default)]
    url_w: String,
    /// Mobile URL
    #[serde(default)]
    url_m: String,
    /// Media name
    #[serde(rename = "Art_Media_Name", default)]
    media_name: String,
    /// Publish time
    #[serde(default)]
    showtime: String,
    /// Comment count
    #[serde(default)]
    commentnum: String,
    /// Image flag
    #[serde(default)]
    image: String,
    /// Image URL
    #[serde(rename = "Art_24Image", default)]
    image_url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct StockInfoItem {
    #[serde(rename = "TOTAL_SHARES", default)]
    total_shares: Option<f64>,
    #[serde(rename = "LISTED_A_SHARES", default)]
    listed_a_shares: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::DataSource;

    #[test]
    fn test_eastmoney_source_creation() {
        let source = EastMoneySource::new().unwrap();
        assert_eq!(source.name(), "eastmoney");
        assert_eq!(source.priority(), 1);
    }

    #[test]
    fn test_eastmoney_default() {
        let source = EastMoneySource::default();
        assert_eq!(source.name(), "eastmoney");
    }
}
