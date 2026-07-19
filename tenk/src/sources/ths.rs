//! THS (同花顺) data source.

use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, COOKIE, HOST, HeaderMap, HeaderValue, REFERER, USER_AGENT,
};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    BondCurrentData, BoardItem, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, NewsArticle,
    NewsCategory, NewsContent, StockCode, StockInfo,
};
use crate::error::{DataError, DataResult};
use crate::request::{RequestConfig, RequestManager};
use crate::util::{
    decode_gb18030, extract_ths_news_content, extract_ths_news_title, is_board_antibot_page,
    kline_period_code, kline_scale, normalize_date_bound, parse_board_html, parse_jsonp,
    parse_kline_records, parse_ths_concept_board_section, parse_ths_industry_board_links,
    SinaKLineRecord,
};
use crate::traits::{
    BondInfoSource, BondMarketSource, BoardMarketSource, DataSource, FundInfoSource,
    FundMarketSource, NewsSource, StockInfoSource, StockMarketSource,
};

/// THS data source.
#[derive(Debug, Clone)]
pub struct THSSource {
    request: RequestManager,
    data_request: RequestManager,
    q_request: RequestManager,
    hq_request: RequestManager,
    chart_request: RequestManager,
    news_request: RequestManager,
}

impl THSSource {
    /// Creates a new THS source.
    pub fn new() -> DataResult<Self> {
        Self::try_new(None)
    }

    pub fn try_new(proxy: Option<&str>) -> DataResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("d.10jqka.com.cn"));
        headers.insert(REFERER, HeaderValue::from_static("http://q.10jqka.com.cn/"));
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

        let config = RequestConfig::default()
            .with_headers(headers)
            .with_proxy_opt(proxy);

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
        data_headers.insert(COOKIE, HeaderValue::from_static("v=1"));

        let data_config = RequestConfig::default()
            .with_headers(data_headers)
            .with_proxy_opt(proxy);

        let mut q_headers = HeaderMap::new();
        q_headers.insert(HOST, HeaderValue::from_static("q.10jqka.com.cn"));
        q_headers.insert(REFERER, HeaderValue::from_static("https://q.10jqka.com.cn/"));
        q_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
        );
        q_headers.insert(COOKIE, HeaderValue::from_static("v=1"));
        q_headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
        );
        let q_config = RequestConfig::default()
            .with_headers(q_headers)
            .with_proxy_opt(proxy);

        let mut hq_headers = HeaderMap::new();
        hq_headers.insert(HOST, HeaderValue::from_static("hq.sinajs.cn"));
        hq_headers.insert(REFERER, HeaderValue::from_static("https://finance.sina.com.cn"));
        hq_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
        );
        let hq_config = RequestConfig::default()
            .with_headers(hq_headers)
            .with_proxy_opt(proxy);

        let chart_config = RequestConfig::default().with_proxy_opt(proxy);

        let mut news_headers = HeaderMap::new();
        news_headers.insert(HOST, HeaderValue::from_static("news.10jqka.com.cn"));
        news_headers.insert(REFERER, HeaderValue::from_static("https://news.10jqka.com.cn/"));
        news_headers.insert(COOKIE, HeaderValue::from_static("v=1"));
        news_headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
        );
        let news_config = RequestConfig::default()
            .with_headers(news_headers)
            .with_proxy_opt(proxy);

        Ok(Self {
            request: RequestManager::new(config)?,
            data_request: RequestManager::new(data_config)?,
            q_request: RequestManager::new(q_config)?,
            hq_request: RequestManager::new(hq_config)?,
            chart_request: RequestManager::new(chart_config)?,
            news_request: RequestManager::new(news_config)?,
        })
    }

    pub fn with_request_manager(request: RequestManager) -> Self {
        Self {
            request: request.clone(),
            data_request: request.clone(),
            q_request: request.clone(),
            hq_request: request.clone(),
            chart_request: request.clone(),
            news_request: request,
        }
    }

    async fn fetch_q_html(&self, url: &str) -> DataResult<String> {
        let response = self.q_request.get(url).await?;
        let bytes = response.bytes().await.map_err(DataError::Network)?;
        Ok(decode_gb18030(&bytes))
    }

    async fn resolve_ths_news_url(&self, news_id: &str) -> DataResult<String> {
        for page in 1..=5u32 {
            let params = [
                ("page", page.to_string()),
                ("tag", String::new()),
                ("track", "website".to_string()),
                ("pagesize", "50".to_string()),
            ];
            #[derive(Deserialize)]
            struct Resp {
                data: Option<RespData>,
            }
            #[derive(Deserialize)]
            struct RespData {
                list: Option<Vec<NewsRow>>,
            }
            #[derive(Deserialize)]
            struct NewsRow {
                seq: Option<String>,
                id: Option<String>,
                url: Option<String>,
            }
            let response: Resp = self
                .news_request
                .get_json_with_params(
                    "https://news.10jqka.com.cn/tapp/news/push/stock/",
                    &params,
                )
                .await?;
            let rows = response.data.and_then(|d| d.list).unwrap_or_default();
            if rows.is_empty() {
                break;
            }
            for row in rows {
                let id = row.seq.or(row.id).unwrap_or_default();
                if id == news_id {
                    if let Some(url) = row.url.filter(|u| !u.is_empty()) {
                        return Ok(url);
                    }
                }
            }
        }
        Err(DataError::custom(format!(
            "THS news article not found: {news_id}"
        )))
    }

    async fn fetch_board_codes(
        &self,
        board: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<(String, String)>> {
        let mut all_codes = Vec::new();
        let mut page = 1u32;

        loop {
            let url = format!(
                "https://q.10jqka.com.cn/index/index/board/{board}/field/code/order/asc/page/{page}/ajax/1/"
            );
            let response = match self.q_request.get(&url).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to fetch THS board page {}: {}", page, e);
                    break;
                }
            };
            let html = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("Failed to read THS board page {}: {}", page, e);
                    break;
                }
            };
            if is_board_antibot_page(&html) {
                if all_codes.is_empty() {
                    return Err(DataError::custom("THS board page blocked by anti-bot"));
                }
                break;
            }
            let batch = parse_board_html(&html);
            if batch.is_empty() {
                break;
            }
            let count = batch.len();
            for (code, name) in batch {
                all_codes.push((code, name));
                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }
            if count < 20 {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }

    fn market_prefix(stock_code: &str) -> &'static str {
        match Exchange::from_stock_code(stock_code) {
            Exchange::SH => "sh",
            Exchange::SZ => "sz",
            Exchange::BJ => "bj",
            Exchange::Unknown => "sh",
        }
    }

    fn full_symbol(stock_code: &str) -> String {
        format!("{}{}", Self::market_prefix(stock_code), stock_code)
    }

    fn board_kline_period(k_type: KLineType) -> DataResult<&'static str> {
        match k_type {
            KLineType::Daily => Ok("01"),
            KLineType::Weekly => Ok("11"),
            KLineType::Monthly => Ok("21"),
            KLineType::Min30 => Ok("30"),
            KLineType::Min60 => Ok("60"),
            other => Err(DataError::not_supported(format!(
                "THS board kline type {other:?}"
            ))),
        }
    }

    fn parse_ths_kline_rows(
        data: &str,
        label_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Vec<MarketData> {
        let start = normalize_date_bound(start_date, "1990-01-01");
        let end = normalize_date_bound(end_date, "2099-12-31");
        let mut result = Vec::new();

        for line in data.split(';') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                continue;
            }

            let trade_date = match NaiveDate::parse_from_str(parts[0], "%Y%m%d") {
                Ok(d) => d,
                Err(_) => continue,
            };

            let trade_date_str = trade_date.format("%Y-%m-%d").to_string();
            if trade_date_str.as_str() < start.as_str() || trade_date_str.as_str() > end.as_str() {
                continue;
            }

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let high: f64 = parts[2].parse().unwrap_or(0.0);
            let low: f64 = parts[3].parse().unwrap_or(0.0);
            let close: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse().unwrap_or(0);
            let amount: f64 = parts[6].parse().unwrap_or(0.0);

            result.push(MarketData {
                stock_code: label_code.to_string(),
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

        result
    }

    async fn fetch_sina_kline(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let scale = kline_scale(k_type)
            .ok_or_else(|| DataError::not_supported(format!("sina fallback kline {k_type:?}")))?;
        let symbol = Self::full_symbol(stock_code);
        let url = "https://money.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_MarketData.getKLineData";
        let params = [
            ("symbol", symbol.as_str()),
            ("scale", &scale.to_string()),
            ("ma", "no"),
            ("datalen", "1023"),
        ];

        debug!("Fetching THS Sina fallback K-line: {} scale={}", stock_code, scale);

        let records: Vec<SinaKLineRecord> = self.chart_request.get_json_with_params(url, &params).await?;
        let intraday = scale < 240;
        Ok(parse_kline_records(
            stock_code,
            &records,
            start_date,
            end_date,
            intraday,
        ))
    }

    fn parse_bond_hq_line(line: &str) -> Option<BondCurrentData> {
        let eq_pos = line.find('=')?;
        if eq_pos < 6 {
            return None;
        }
        let bond_code = line[eq_pos - 6..eq_pos].to_string();
        let quote_start = line.find('"')? + 1;
        let quote_end = line.rfind('"')?;
        let parts: Vec<&str> = line[quote_start..quote_end].split(',').collect();
        if parts.len() < 10 {
            return None;
        }
        let bond_name = parts[0].to_string();
        let open: f64 = parts[1].parse().unwrap_or(0.0);
        let pre_close: f64 = parts[2].parse().unwrap_or(0.0);
        let price: f64 = parts[3].parse().unwrap_or(0.0);
        let high: f64 = parts[4].parse().unwrap_or(0.0);
        let low: f64 = parts[5].parse().unwrap_or(0.0);
        let volume: u64 = parts[8].parse().unwrap_or(0);
        let amount: f64 = parts[9].parse().unwrap_or(0.0);
        let change = price - pre_close;
        let change_pct = if pre_close > 0.0 {
            (change / pre_close) * 100.0
        } else {
            0.0
        };
        Some(BondCurrentData {
            bond_code,
            bond_name,
            price,
            open,
            high,
            low,
            pre_close,
            change,
            change_pct,
            volume,
            amount,
        })
    }
}

impl Default for THSSource {
    /// Creates default THS source.
    fn default() -> Self {
        Self::new().expect("Failed to create THSSource")
    }
}

/// Bond list API response.
#[derive(Debug, Deserialize)]
struct BondCodeResponse {
    /// Status message
    status_msg: String,
    /// Bond items
    #[serde(default)]
    list: Vec<THSBondItem>,
}

/// Bond item from THS API.
#[derive(Debug, Deserialize)]
struct THSBondItem {
    /// Bond code
    bond_code: String,
    /// Bond name
    bond_name: String,
    /// Stock code
    #[serde(default)]
    code: String,
    /// Stock name
    #[serde(default)]
    name: String,
    /// Subscription date
    #[serde(default)]
    sub_date: Option<String>,
    /// Issue total
    #[serde(default)]
    issue_total: Option<String>,
    /// Listing date
    #[serde(default)]
    listing_date: Option<String>,
    /// Expiration date
    #[serde(default)]
    expire_date: Option<String>,
    /// Conversion price
    #[serde(default)]
    price: Option<String>,
}

/// K-line API response.
#[derive(Debug, Deserialize)]
struct KLineResponse {
    /// Total records
    #[serde(deserialize_with = "deserialize_total")]
    total: u32,
    /// K-line data string
    #[serde(default)]
    data: String,
}

/// Deserializes a value that can be either a number or a string.
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
    /// Returns the source name.
    fn name(&self) -> &'static str {
        "ths"
    }

    /// Returns the source priority.
    fn priority(&self) -> u8 {
        3
    }

    /// Checks if the source is available.
    async fn is_available(&self) -> bool {
        self.data_request
            .get("https://data.10jqka.com.cn/ipo/kzz/")
            .await
            .is_ok()
    }
}

#[async_trait]
impl BondInfoSource for THSSource {
    /// Fetches all available convertible bond codes.
    async fn get_all_bond_codes(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<ConvertibleBondCode>> {
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

        let mut bonds: Vec<ConvertibleBondCode> = data
            .list
            .into_iter()
            .map(|item| {
                let sub_date = item
                    .sub_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let listing_date = item
                    .listing_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let expire_date = item
                    .expire_date
                    .as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let issue_amount = item
                    .issue_total
                    .as_ref()
                    .and_then(|s| s.parse::<f64>().ok().map(|v| v * 100_000_000.0));
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

        if let Some(lim) = limit {
            bonds.truncate(lim);
        }

        Ok(bonds)
    }
}

#[async_trait]
impl BondMarketSource for THSSource {
    /// Fetches real-time bond quotes.
    async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        let codes: Vec<String> = match bond_codes {
            Some(items) if !items.is_empty() => items.iter().map(|c| c.to_string()).collect(),
            _ => self
                .get_all_bond_codes(None)
                .await?
                .into_iter()
                .map(|b| b.bond_code)
                .collect(),
        };

        if codes.is_empty() {
            return Ok(Vec::new());
        }

        let symbols: String = codes
            .iter()
            .map(|c| format!("{}{}", Self::market_prefix(c), c))
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("https://hq.sinajs.cn/list={symbols}");
        let response = self.hq_request.get(&url).await?;
        let bytes = response.bytes().await.map_err(DataError::Network)?;
        let text = decode_gb18030(&bytes);
        Ok(text
            .lines()
            .filter_map(Self::parse_bond_hq_line)
            .collect())
    }
}

#[async_trait]
impl StockMarketSource for THSSource {
    /// Fetches historical K-line market data.
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        if matches!(k_type, KLineType::Min5 | KLineType::Min15) {
            return self
                .fetch_sina_kline(stock_code, start_date, end_date, k_type)
                .await;
        }

        let k_code = kline_period_code(k_type)?;
        let url = format!("http://d.10jqka.com.cn/v6/line/hs_{stock_code}/{k_code}/last36000.js");

        debug!("Fetching stock market data from THS: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: KLineResponse = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS stock response: {e}")))?;

        if data.total == 0 || data.data.is_empty() {
            return Ok(Vec::new());
        }

        Ok(Self::parse_ths_kline_rows(
            &data.data,
            stock_code,
            start_date,
            end_date,
        ))
    }

    /// Fetches real-time market quotes.
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

            let json_str = match parse_jsonp(&text) {
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
            let volume: u64 = stock_data.get("13").and_then(|v| v.as_u64()).unwrap_or(0);
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

    /// Fetches intraday minute-level data.
    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let url = format!("http://d.10jqka.com.cn/v6/time/hs_{stock_code}/last.js");
        debug!("Fetching stock minute data from THS: {}", stock_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = parse_jsonp(&text)
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
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        let rows = self.fetch_board_codes("hs", limit).await?;
        Ok(rows
            .into_iter()
            .map(|(code, name)| StockCode {
                stock_code: code.clone(),
                short_name: name,
                exchange: Exchange::from_stock_code(&code),
                list_date: None,
            })
            .collect())
    }

    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let url = format!("http://d.10jqka.com.cn/v6/line/hs_{stock_code}/01/today.js");
        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;
        let json_str = parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS stock info response"))?;
        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS stock info JSON: {e}")))?;
        let key = format!("hs_{stock_code}");
        let stock = data
            .get(&key)
            .ok_or_else(|| DataError::custom("No stock info available"))?;
        let short_name = stock
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(StockInfo {
            stock_code: stock_code.to_string(),
            full_name: String::new(),
            short_name,
            exchange: Exchange::from_stock_code(stock_code),
            industry: None,
            total_shares: None,
            circulating_shares: None,
            list_date: None,
        })
    }
}

#[async_trait]
impl FundInfoSource for THSSource {
    /// Fetches all available ETF codes.
    async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        let rows = self.fetch_board_codes("fund", limit).await?;
        Ok(rows
            .into_iter()
            .map(|(code, name)| ETFCode {
                fund_code: code.clone(),
                short_name: name,
                exchange: Exchange::from_stock_code(&code),
                net_value: None,
            })
            .collect())
    }
}

#[async_trait]
impl FundMarketSource for THSSource {
    /// Fetches historical ETF K-line market data.
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let k_code = kline_period_code(k_type)?;
        let url = format!("http://d.10jqka.com.cn/v6/line/hs_{fund_code}/{k_code}/last36000.js");

        debug!("Fetching ETF market data from THS: {}", fund_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS JSONP response"))?;

        let data: KLineResponse = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS ETF response: {e}")))?;

        if data.total == 0 || data.data.is_empty() {
            return Ok(Vec::new());
        }

        let start = normalize_date_bound(start_date, "1990-01-01");
        let end = normalize_date_bound(end_date, "2099-12-31");

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
            if trade_date_str.as_str() < start.as_str() || trade_date_str.as_str() > end.as_str() {
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

    /// Fetches real-time ETF quotes.
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

            let json_str = match parse_jsonp(&text) {
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
            let volume: u64 = etf_data.get("13").and_then(|v| v.as_u64()).unwrap_or(0);
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

    /// Fetches intraday ETF minute-level data.
    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let url = format!("http://d.10jqka.com.cn/v6/time/hs_{fund_code}/last.js");
        debug!("Fetching ETF minute data from THS: {}", fund_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        let json_str = parse_jsonp(&text)
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

        let trade_date_str = etf_data.get("date").and_then(|v| v.as_str()).unwrap_or("");

        let data_str = etf_data.get("data").and_then(|v| v.as_str()).unwrap_or("");

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

#[async_trait]
impl BoardMarketSource for THSSource {
    async fn get_industry_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        let html = self.fetch_q_html("https://q.10jqka.com.cn/thshy/").await?;
        let mut items: Vec<BoardItem> = parse_ths_industry_board_links(&html)
            .into_iter()
            .map(|(code, name)| BoardItem {
                board_code: code,
                board_name: name,
                price: 0.0,
                change_pct: 0.0,
            })
            .collect();
        if let Some(lim) = limit {
            items.truncate(lim);
        }
        Ok(items)
    }

    async fn get_concept_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        let html = self.fetch_q_html("https://q.10jqka.com.cn/gn/").await?;
        let mut items: Vec<BoardItem> = parse_ths_concept_board_section(&html)
            .into_iter()
            .map(|(code, name, change_pct, _)| BoardItem {
                board_code: code,
                board_name: name,
                price: 0.0,
                change_pct,
            })
            .collect();
        if let Some(lim) = limit {
            items.truncate(lim);
        }
        Ok(items)
    }

    async fn get_board_market(
        &self,
        board_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let k_code = Self::board_kline_period(k_type)?;
        let normalized = board_code.trim_start_matches("BK").trim_start_matches("bk");
        let url = format!("http://d.10jqka.com.cn/v4/line/bk_{normalized}/{k_code}/last.js");

        debug!("Fetching board market data from THS: {}", board_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let json_str = parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS board JSONP response"))?;

        let data: KLineResponse = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS board response: {e}")))?;

        if data.total == 0 || data.data.is_empty() {
            return Ok(Vec::new());
        }

        Ok(Self::parse_ths_kline_rows(
            &data.data,
            board_code,
            start_date,
            end_date,
        ))
    }

    async fn get_board_constituents(
        &self,
        board_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<StockCode>> {
        let normalized = board_code.trim_start_matches("BK").trim_start_matches("bk");
        let url = format!("http://d.10jqka.com.cn/v2/blockrank/{normalized}/199112/d1000.js");

        debug!("Fetching THS board constituents: {}", board_code);

        let response = self.request.get(&url).await?;
        let text = response.text().await.map_err(DataError::Network)?;
        let json_str = parse_jsonp(&text)
            .ok_or_else(|| DataError::custom("Failed to parse THS board rank JSONP"))?;

        #[derive(Deserialize)]
        struct RankResp {
            items: Option<Vec<RankItem>>,
        }
        #[derive(Deserialize)]
        struct RankItem {
            #[serde(rename = "5")]
            code: Option<String>,
            #[serde(rename = "55")]
            name: Option<String>,
        }

        let data: RankResp = serde_json::from_str(json_str)
            .map_err(|e| DataError::custom(format!("Failed to parse THS board rank: {e}")))?;
        let items = data.items.unwrap_or_default();
        let mut result = Vec::new();
        for item in items {
            let code = item.code.unwrap_or_default();
            if code.len() != 6 {
                continue;
            }
            let exchange = Exchange::from_stock_code(&code);
            result.push(StockCode {
                stock_code: code,
                short_name: item.name.unwrap_or_default(),
                exchange,
                list_date: None,
            });
            if let Some(lim) = limit {
                if result.len() >= lim {
                    break;
                }
            }
        }
        Ok(result)
    }
}

#[async_trait]
impl NewsSource for THSSource {
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let _ = category;
        let url = "https://news.10jqka.com.cn/tapp/news/push/stock/";
        let params = [
            ("page", page.to_string()),
            ("tag", String::new()),
            ("track", "website".to_string()),
            ("pagesize", limit.max(1).min(50).to_string()),
        ];

        debug!("Fetching THS news page {}", page);

        #[derive(Deserialize)]
        struct Resp {
            data: Option<RespData>,
        }
        #[derive(Deserialize)]
        struct RespData {
            list: Option<Vec<NewsRow>>,
        }
        #[derive(Deserialize)]
        struct NewsRow {
            id: Option<String>,
            seq: Option<String>,
            title: String,
            digest: Option<String>,
            url: Option<String>,
            #[serde(rename = "appUrl")]
            app_url: Option<String>,
            ctime: Option<String>,
            source: Option<String>,
            #[serde(rename = "picUrl")]
            pic_url: Option<String>,
        }

        let response: Resp = self.news_request.get_json_with_params(url, &params).await?;
        let rows = response.data.and_then(|d| d.list).unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let id = row.seq.or(row.id)?;
                let publish_time = row
                    .ctime
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|ts| Utc.timestamp_opt(ts, 0).single())
                    .flatten()?;
                Some(NewsArticle {
                    id,
                    title: row.title,
                    digest: row.digest.unwrap_or_default(),
                    url: row.url.unwrap_or_default(),
                    url_mobile: row.app_url,
                    source: row.source.unwrap_or_else(|| "THS".to_string()),
                    publish_time,
                    category: NewsCategory::Finance,
                    comment_count: 0,
                    has_image: row.pic_url.as_ref().is_some_and(|s| !s.is_empty()),
                    image_url: row.pic_url.filter(|s| !s.is_empty()),
                })
            })
            .collect())
    }

    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        let article_url = self.resolve_ths_news_url(news_id).await?;
        debug!("Fetching THS news content: {}", article_url);
        let response = self.news_request.get(&article_url).await?;
        let html = response.text().await.map_err(DataError::Network)?;
        if html.contains("Nginx forbidden") {
            return Err(DataError::custom("THS news content blocked"));
        }
        let title = extract_ths_news_title(&html).unwrap_or_default();
        let (body_html, body_text) = extract_ths_news_content(&html)
            .ok_or_else(|| DataError::custom("Failed to parse THS news content"))?;
        Ok(NewsContent {
            id: news_id.to_string(),
            title,
            description: body_text.clone(),
            body_html,
            body_text,
            source: "THS".to_string(),
            author: None,
            publish_time: Utc::now(),
            related_stocks: Vec::new(),
            images: Vec::new(),
        })
    }

    async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Ok(Vec::new());
        }
        let scan_pages = (page + 2).min(10);
        let mut matched = Vec::new();
        for scan_page in 1..=scan_pages {
            let batch = self
                .get_news(NewsCategory::Finance, scan_page, 50)
                .await?;
            for article in batch {
                if article.title.contains(keyword) || article.digest.contains(keyword) {
                    matched.push(article);
                }
            }
        }
        let start = ((page.saturating_sub(1)) as usize) * limit as usize;
        let end = start + limit as usize;
        Ok(matched.into_iter().skip(start).take(end).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jsonp() {
        let text = r#"callback({"data": "test"})"#;
        let result = parse_jsonp(text);
        assert_eq!(result, Some(r#"{"data": "test"}"#));
    }

    #[test]
    fn test_parse_bond_hq_line() {
        let line = r#"var hq_str_sh113050="南银转债,0.000,144.967,144.967,0.000,0.000,0.000,0.000,0,0.000,0,0.000";"#;
        let bond = THSSource::parse_bond_hq_line(line).unwrap();
        assert_eq!(bond.bond_code, "113050");
        assert_eq!(bond.bond_name, "南银转债");
        assert_eq!(bond.price, 144.967);
    }

    #[test]
    fn test_kline_period_code() {
        assert_eq!(kline_period_code(KLineType::Daily).unwrap(), "01");
        assert_eq!(kline_period_code(KLineType::Min30).unwrap(), "30");
        assert_eq!(kline_period_code(KLineType::Min60).unwrap(), "60");
        assert!(kline_period_code(KLineType::Min5).is_err());
        assert!(kline_period_code(KLineType::Quarterly).is_err());
    }

    #[test]
    fn test_parse_jsonp_empty_object() {
        assert_eq!(parse_jsonp(r#"cb({})"#), Some("{}"));
    }

    #[test]
    fn test_parse_ths_kline_rows() {
        let data = "20250102,10.0,11.0,9.5,10.5,1000,50000.0,,,,0;20250103,10.5,11.5,10.0,11.0,1200,60000.0,,,,0";
        let rows = THSSource::parse_ths_kline_rows(data, "600519", None, None);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].close, 11.0);
        assert!((rows[1].change_pct - 4.761904761904762).abs() < 0.01);
    }

    #[test]
    fn test_board_kline_period() {
        assert_eq!(THSSource::board_kline_period(KLineType::Daily).unwrap(), "01");
        assert!(THSSource::board_kline_period(KLineType::Min5).is_err());
    }
}
