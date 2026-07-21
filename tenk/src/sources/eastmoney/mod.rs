//! East Money data source.

mod stock;
mod bond;
mod news;
mod extended;
mod market_extra;
mod derivatives;
mod search;

use std::time::Duration;

use chrono::FixedOffset;
use serde::Deserialize;
use crate::error::DataResult;
use crate::request::{RequestConfig, RequestManager};

/// East Money data source.
#[derive(Debug, Clone)]
pub struct EastMoneySource {
    request: RequestManager,
    history_request: RequestManager,
}

impl EastMoneySource {
    pub fn new() -> DataResult<Self> {
        Self::try_new(None)
    }

    pub fn try_new(proxy: Option<&str>) -> DataResult<Self> {
        use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};

        let mut headers = HeaderMap::new();
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://quote.eastmoney.com/"),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        let config = RequestConfig::default()
            .with_proxy_opt(proxy)
            .with_headers(headers.clone());
        let request = RequestManager::new(config)?;
        let history_config = RequestConfig::default()
            .with_proxy_opt(proxy)
            .with_headers(headers)
            .with_retries(1)
            .with_timeout(Duration::from_secs(8));
        let history_request = RequestManager::new(history_config)?;
        Ok(Self {
            request,
            history_request,
        })
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        let mut history_config = request.config().clone();
        history_config.max_retries = 1;
        history_config.timeout = Duration::from_secs(8);
        Self {
            history_request: RequestManager::new(history_config)
                .expect("Failed to create East Money history request manager"),
            request,
        }
    }
}

impl Default for EastMoneySource {
    fn default() -> Self {
        Self::new().expect("Failed to create EastMoneySource")
    }
}

const CLIST_URL: &str = "https://push2delay.eastmoney.com/api/qt/clist/get";
const KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const KLINE_FALLBACK_URL: &str = "https://push2delay.eastmoney.com/api/qt/stock/kline/get";

impl EastMoneySource {
    pub(crate) async fn fetch_kline_lines(
        &self,
        params: &[(&str, &str)],
    ) -> DataResult<Vec<String>> {
        for url in [KLINE_URL, KLINE_FALLBACK_URL] {
            let response: KLineResponse = match self.request.get_json_with_params(url, params).await {
                Ok(response) => response,
                Err(_) => continue,
            };
            let klines = response
                .data
                .and_then(|data| data.klines)
                .unwrap_or_default();
            if !klines.is_empty() {
                return Ok(klines);
            }
        }

        Ok(Vec::new())
    }
}

fn beijing_tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).unwrap()
}

pub(crate) fn deserialize_opt_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptF64Visitor;

    impl<'de> Visitor<'de> for OptF64Visitor {
        type Value = Option<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, string, or number")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.parse().ok())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.parse().ok())
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }
    }

    deserializer.deserialize_any(OptF64Visitor)
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

    #[test]
    fn test_deserialize_opt_f64() {
        #[derive(Deserialize)]
        struct Row {
            #[serde(default, deserialize_with = "deserialize_opt_f64")]
            value: Option<f64>,
        }

        let missing: Row = serde_json::from_str(r#"{"value":"-"}"#).unwrap();
        assert!(missing.value.is_none());

        let present: Row = serde_json::from_str(r#"{"value":1.23}"#).unwrap();
        assert_eq!(present.value, Some(1.23));
    }
}
