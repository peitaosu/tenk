//! THS (同花顺) data source.

use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, COOKIE, HOST, REFERER, USER_AGENT};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, KLineType, MarketData, MinuteData, StockCode, StockInfo,
};
use crate::error::{DataError, DataResult};
use crate::request::{RequestConfig, RequestManager};
use crate::traits::{
    BondInfoSource, BondMarketSource, DataSource, FundInfoSource, FundMarketSource,
    StockInfoSource, StockMarketSource,
};

#[derive(Debug, Clone)]
pub struct THSSource {
    request: RequestManager,
    data_request: RequestManager,
}

impl THSSource {
    pub fn new() -> DataResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("d.10jqka.com.cn"));
        headers.insert(
            REFERER,
            HeaderValue::from_static("http://q.10jqka.com.cn/"),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2"),
        );

        let config = RequestConfig::default().with_headers(headers);

        let mut data_headers = HeaderMap::new();
        data_headers.insert(HOST, HeaderValue::from_static("data.10jqka.com.cn"));
        data_headers.insert(
            REFERER,
            HeaderValue::from_static("http://data.10jqka.com.cn/ipo/bond/"),
        );
        data_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
        );
        data_headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
        );
        data_headers.insert(
            COOKIE,
            HeaderValue::from_static("v=1"),
        );

        let data_config = RequestConfig::default().with_headers(data_headers);

        Ok(Self {
            request: RequestManager::new(config)?,
            data_request: RequestManager::new(data_config)?,
        })
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        Self {
            request: request.clone(),
            data_request: request,
        }
    }

    fn kline_type_to_code(k_type: KLineType) -> &'static str {
        match k_type {
            KLineType::Daily => "01",
            KLineType::Weekly => "11",
            KLineType::Monthly => "21",
            _ => "01",
        }
    }

    fn parse_jsonp(text: &str) -> Option<&str> {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        if start < end {
            Some(&text[start..=end])
        } else {
            None
        }
    }
}

impl Default for THSSource {
    fn default() -> Self {
        Self::new().expect("Failed to create THSSource")
    }
}

#[derive(Debug, Deserialize)]
struct BondCodeResponse {
    status_msg: String,
    #[serde(default)]
    list: Vec<THSBondItem>,
}

#[derive(Debug, Deserialize)]
struct THSBondItem {
    bond_code: String,
    bond_name: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    sub_date: Option<String>,
    #[serde(default)]
    issue_total: Option<String>,
    #[serde(default)]
    listing_date: Option<String>,
    #[serde(default)]
    expire_date: Option<String>,
    #[serde(default)]
    price: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KLineResponse {
    #[serde(deserialize_with = "deserialize_total")]
    total: u32,
    #[serde(default)]
    data: String,
}

fn deserialize_total<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Visitor;
    use std::fmt;

    struct TotalVisitor;

    impl<'de> Visitor<'de> for TotalVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a number or string")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v as u32)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v as u32)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(TotalVisitor)
}

#[async_trait]
impl DataSource for THSSource {
    fn name(&self) -> &'static str {
        "ths"
    }

    fn priority(&self) -> u8 {
        3
    }

    async fn is_available(&self) -> bool {
        self.data_request
            .get("https://data.10jqka.com.cn/ipo/kzz/")
            .await
            .is_ok()
    }
}

#[async_trait]
impl BondInfoSource for THSSource {
    async fn get_all_bond_codes(&self) -> DataResult<Vec<ConvertibleBondCode>> {
        let url = "https://data.10jqka.com.cn/ipo/kzz/";
        debug!("Fetching bond codes from THS");

        let response = self.data_request.get(url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let data: BondCodeResponse = serde_json::from_str(&text)
            .map_err(|e| DataError::custom(format!("Failed to parse THS bond response: {e}")))?;

        if data.status_msg != "ok" {
            return Err(DataError::custom(format!(
                "THS API returned error: {}",
                data.status_msg
            )));
        }

        let bonds: Vec<ConvertibleBondCode> = data
            .list
            .into_iter()
            .map(|item| {
                let sub_date = item.sub_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                });
                let listing_date = item.listing_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                });
                let expire_date = item.expire_date.as_ref().and_then(|s| {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                });
                let issue_amount = item.issue_total.as_ref().and_then(|s| {
                    s.parse::<f64>().ok().map(|v| v * 100_000_000.0)
                });
                let convert_price = item.price.as_ref().and_then(|s| s.parse().ok());

                ConvertibleBondCode {
                    bond_code: item.bond_code,
                    bond_name: item.bond_name,
                    stock_code: item.code,
                    short_name: item.name,
                    sub_date,
                    issue_amount,
                    listing_date,
                    expire_date,
                    convert_price,
                }
            })
            .collect();

        Ok(bonds)
    }
}

#[async_trait]
impl BondMarketSource for THSSource {
    async fn get_bond_current(
        &self,
        _bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        Err(DataError::not_supported("ths: get_bond_current"))
    }
}

#[async_trait]
impl StockMarketSource for THSSource {
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let k_code = Self::kline_type_to_code(k_type);
        let url = format!("http://d.10jqka.com.cn/v6/line/hs_{stock_code}/{k_code}/last36000.js");

        debug!("Fetching stock market data from THS: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = Self::parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: KLineResponse = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS stock response: {e}")))?;

        if data.total == 0 || data.data.is_empty() {
            return Ok(Vec::new());
        }

        let start = start_date.unwrap_or("1990-01-01");
        let end = end_date.unwrap_or("2099-12-31");

        let mut result = Vec::new();

        for line in data.data.split(';') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                continue;
            }

            let date_str = parts[0];
            let trade_date = match NaiveDate::parse_from_str(date_str, "%Y%m%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let trade_date_str = trade_date.format("%Y-%m-%d").to_string();
            if trade_date_str.as_str() < start || trade_date_str.as_str() > end {
                continue;
            }

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let high: f64 = parts[2].parse().unwrap_or(0.0);
            let low: f64 = parts[3].parse().unwrap_or(0.0);
            let close: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse().unwrap_or(0);
            let amount: f64 = parts[6].parse().unwrap_or(0.0);

            result.push(MarketData {
                stock_code: stock_code.to_string(),
                trade_time: Utc.from_utc_datetime(&trade_date.and_hms_opt(15, 0, 0).unwrap()),
                trade_date,
                open,
                close,
                high,
                low,
                volume,
                amount,
                change: 0.0,
                change_pct: 0.0,
                turnover_ratio: 0.0,
                pre_close: 0.0,
            });
        }

        if result.len() > 1 {
            for i in 1..result.len() {
                let prev_close = result[i - 1].close;
                let curr_close = result[i].close;
                result[i].pre_close = prev_close;
                result[i].change = curr_close - prev_close;
                result[i].change_pct = if prev_close > 0.0 {
                    (curr_close - prev_close) / prev_close * 100.0
                } else {
                    0.0
                };
            }
        }

        Ok(result)
    }

    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let mut result = Vec::new();

        for code in stock_codes {
            let url = format!("http://d.10jqka.com.cn/v6/line/hs_{code}/01/today.js");

            let response = match self.request.get(&url).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch stock current for {}: {}", code, e);
                    continue;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let json_str = match Self::parse_jsonp(&text) {
                Some(s) => s,
                None => continue,
            };

            let data: serde_json::Value = match serde_json::from_str(json_str) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let stock_data = match data.get(format!("hs_{code}")) {
                Some(d) => d,
                None => continue,
            };

            let price: f64 = stock_data
                .get("11")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let open: f64 = stock_data
                .get("7")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let high: f64 = stock_data
                .get("8")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let low: f64 = stock_data
                .get("9")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let volume: u64 = stock_data
                .get("13")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let amount: f64 = stock_data
                .get("19")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let name = stock_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            result.push(CurrentMarketData {
                stock_code: code.to_string(),
                short_name: name,
                price,
                change: 0.0,
                change_pct: 0.0,
                volume,
                amount,
                open: Some(open),
                high: Some(high),
                low: Some(low),
                pre_close: None,
            });
        }

        Ok(result)
    }

    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let url = format!("http://d.10jqka.com.cn/v6/time/hs_{stock_code}/last.js");
        debug!("Fetching stock minute data from THS: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = Self::parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS minute response: {e}")))?;

        let stock_data = data
            .get(format!("hs_{stock_code}"))
            .ok_or_else(|| DataError::custom("Stock data not found in response"))?;

        let pre_close: f64 = stock_data
            .get("pre")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let trade_date_str = stock_data
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let data_str = stock_data
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if data_str.is_empty() {
            return Ok(Vec::new());
        }

        let today = NaiveDate::parse_from_str(trade_date_str, "%Y%m%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        let mut result = Vec::new();

        for line in data_str.split(';') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                continue;
            }

            let time_str = parts[0];
            let price: f64 = parts[1].parse().unwrap_or(0.0);
            let amount: f64 = parts[2].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[3].parse().unwrap_or(0.0);
            let volume: u64 = parts[4].parse().unwrap_or(0);

            let trade_time = chrono::NaiveTime::parse_from_str(time_str, "%H%M")
                .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                .unwrap_or_else(|_| Utc::now());

            let change = price - pre_close;
            let change_pct = if pre_close > 0.0 {
                (change / pre_close * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };

            result.push(MinuteData {
                stock_code: stock_code.to_string(),
                trade_time,
                price,
                change,
                change_pct,
                volume,
                avg_price,
                amount,
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl StockInfoSource for THSSource {
    async fn get_all_codes(&self) -> DataResult<Vec<StockCode>> {
        Err(DataError::not_supported("ths: get_all_codes"))
    }

    async fn get_stock_info(&self, _stock_code: &str) -> DataResult<StockInfo> {
        Err(DataError::not_supported("ths: get_stock_info"))
    }
}

#[async_trait]
impl FundInfoSource for THSSource {
    async fn get_all_etf_codes(&self) -> DataResult<Vec<ETFCode>> {
        Err(DataError::not_supported("ths: get_all_etf_codes"))
    }
}

#[async_trait]
impl FundMarketSource for THSSource {
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let k_code = Self::kline_type_to_code(k_type);
        let url = format!(
            "http://d.10jqka.com.cn/v6/line/hs_{fund_code}/{k_code}/last36000.js"
        );

        debug!("Fetching ETF market data from THS: {}", fund_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = Self::parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: KLineResponse = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS ETF response: {e}")))?;

        if data.total == 0 || data.data.is_empty() {
            return Ok(Vec::new());
        }

        let start = start_date.unwrap_or("1990-01-01");
        let end = end_date.unwrap_or("2099-12-31");

        let mut result = Vec::new();

        for line in data.data.split(';') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                continue;
            }

            let date_str = parts[0];
            let trade_date = match NaiveDate::parse_from_str(date_str, "%Y%m%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let trade_date_str = trade_date.format("%Y-%m-%d").to_string();
            if trade_date_str.as_str() < start || trade_date_str.as_str() > end {
                continue;
            }

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let high: f64 = parts[2].parse().unwrap_or(0.0);
            let low: f64 = parts[3].parse().unwrap_or(0.0);
            let close: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse().unwrap_or(0);
            let amount: f64 = parts[6].parse().unwrap_or(0.0);

            result.push(ETFMarketData {
                fund_code: fund_code.to_string(),
                trade_time: Utc.from_utc_datetime(&trade_date.and_hms_opt(15, 0, 0).unwrap()),
                trade_date,
                open,
                close,
                high,
                low,
                volume,
                amount,
                change: None,
                change_pct: None,
            });
        }

        if result.len() > 1 {
            for i in 1..result.len() {
                let prev_close = result[i - 1].close;
                let curr_close = result[i].close;
                result[i].change = Some(curr_close - prev_close);
                result[i].change_pct = Some((curr_close - prev_close) / prev_close * 100.0);
            }
        }

        Ok(result)
    }

    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        let mut result = Vec::new();

        for code in fund_codes {
            let url = format!("http://d.10jqka.com.cn/v6/line/hs_{code}/01/today.js");

            let response = match self.request.get(&url).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch ETF current for {}: {}", code, e);
                    continue;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let json_str = match Self::parse_jsonp(&text) {
                Some(s) => s,
                None => continue,
            };

            let data: serde_json::Value = match serde_json::from_str(json_str) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let etf_data = match data.get(format!("hs_{code}")) {
                Some(d) => d,
                None => continue,
            };

            let price: f64 = etf_data
                .get("11")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let open: f64 = etf_data
                .get("7")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let high: f64 = etf_data
                .get("8")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let low: f64 = etf_data
                .get("9")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let volume: u64 = etf_data
                .get("13")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let amount: f64 = etf_data
                .get("19")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let name = etf_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            result.push(ETFCurrentData {
                fund_code: code.to_string(),
                short_name: name,
                price,
                change: None,
                change_pct: None,
                volume,
                amount,
                open: Some(open),
                high: Some(high),
                low: Some(low),
            });
        }

        Ok(result)
    }

    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let url = format!("http://d.10jqka.com.cn/v6/time/hs_{fund_code}/last.js");
        debug!("Fetching ETF minute data from THS: {}", fund_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = Self::parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS minute response: {e}")))?;

        let etf_data = data
            .get(format!("hs_{fund_code}"))
            .ok_or_else(|| DataError::custom("ETF data not found in response"))?;

        let pre_close: f64 = etf_data
            .get("pre")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let trade_date_str = etf_data
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let data_str = etf_data
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if data_str.is_empty() {
            return Ok(Vec::new());
        }

        let today = NaiveDate::parse_from_str(trade_date_str, "%Y%m%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        let mut result = Vec::new();

        for line in data_str.split(';') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                continue;
            }

            let time_str = parts[0];
            let price: f64 = parts[1].parse().unwrap_or(0.0);
            let amount: f64 = parts[2].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[3].parse().unwrap_or(0.0);
            let volume: u64 = parts[4].parse().unwrap_or(0);

            let trade_time = chrono::NaiveTime::parse_from_str(time_str, "%H%M")
                .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                .unwrap_or_else(|_| Utc::now());

            let change = price - pre_close;
            let change_pct = if pre_close > 0.0 {
                (change / pre_close * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };

            result.push(ETFMinuteData {
                fund_code: fund_code.to_string(),
                trade_time,
                price,
                change,
                change_pct,
                volume,
                avg_price,
                amount,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jsonp() {
        let text = r#"callback({"data": "test"})"#;
        let result = THSSource::parse_jsonp(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), r#"{"data": "test"}"#);
    }

    #[test]
    fn test_kline_type_to_code() {
        assert_eq!(THSSource::kline_type_to_code(KLineType::Daily), "01");
        assert_eq!(THSSource::kline_type_to_code(KLineType::Weekly), "11");
        assert_eq!(THSSource::kline_type_to_code(KLineType::Monthly), "21");
    }
}
