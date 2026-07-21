use super::*;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    CurrentMarketData, ETFCode,
    ETFCurrentData, ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, OrderBookData, StockCode, StockInfo, StockSearchHit, TickData,
};
use crate::error::DataResult;
use crate::util::{parse_cn_market_time, parse_order_book_from_fields, parse_tick_details, parse_trade_date};
use crate::traits::{
    DataSource, FundInfoSource, FundMarketSource, StockInfoSource, StockMarketSource,
};

#[async_trait]
impl DataSource for EastMoneySource {
    /// Returns the source name.
    fn name(&self) -> &'static str {
        "eastmoney"
    }

    /// Returns the source priority.
    fn priority(&self) -> u8 {
        1
    }

    /// Checks if the source is available.
    async fn is_available(&self) -> bool {
        self.request
            .get("https://push2delay.eastmoney.com/api/qt/ulist.np/get?fltt=2&secids=1.600519&fields=f12&ut=fa5fd1943c7b386f172d6893dbfba10b")
            .await
            .is_ok()
    }
}

#[async_trait]
impl StockMarketSource for EastMoneySource {
    /// Fetches historical K-line market data.
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        self.get_market_symbol(
            &StockCode::with_inferred_exchange(stock_code),
            start_date,
            end_date,
            k_type,
        )
        .await
    }

    async fn get_market_symbol(
        &self,
        symbol: &StockCode,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let stock_code = symbol.stock_code.as_str();
        let secid = symbol.exchange.eastmoney_secid(stock_code);
        let klt = k_type.to_api_value();

        let start = start_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| "19900101".to_string());
        let end = end_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
            ),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", &klt.to_string()),
            ("fqt", "1"),
            ("secid", &secid),
            ("beg", &start),
            ("end", &end),
        ];

        let klines = self.fetch_kline_lines(&params).await?;

        if klines.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(klines.len());

        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 11 {
                continue;
            }

            let trade_date = parse_trade_date(parts[0]);

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let close: f64 = parts[2].parse().unwrap_or(0.0);
            let high: f64 = parts[3].parse().unwrap_or(0.0);
            let low: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64 * 100;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let change_pct: f64 = parts[8].parse().unwrap_or(0.0);
            let change: f64 = parts[9].parse().unwrap_or(0.0);
            let turnover_ratio: f64 = parts[10].parse().unwrap_or(0.0);
            let pre_close = close - change;

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
                change,
                change_pct,
                turnover_ratio,
                pre_close: (pre_close * 100.0).round() / 100.0,
            });
        }

        Ok(result)
    }

    /// Fetches real-time market quotes.
    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let symbols: Vec<StockCode> = stock_codes
            .iter()
            .map(|code| StockCode::with_inferred_exchange(*code))
            .collect();
        self.get_market_current_symbols(&symbols).await
    }

    async fn get_market_current_symbols(
        &self,
        symbols: &[StockCode],
    ) -> DataResult<Vec<CurrentMarketData>> {
        self.fetch_current_quotes(symbols).await
    }

    /// Fetches intraday minute-level data.
    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        self.get_market_min_symbol(&StockCode::with_inferred_exchange(stock_code))
            .await
    }

    async fn get_market_min_days(&self, stock_code: &str, ndays: u32) -> DataResult<Vec<MinuteData>> {
        self.get_market_min_days_symbol(&StockCode::with_inferred_exchange(stock_code), ndays)
            .await
    }

    async fn get_market_min_symbol(&self, symbol: &StockCode) -> DataResult<Vec<MinuteData>> {
        self.fetch_stock_minute_trends(symbol, 1).await
    }

    async fn get_market_min_days_symbol(
        &self,
        symbol: &StockCode,
        ndays: u32,
    ) -> DataResult<Vec<MinuteData>> {
        self.fetch_stock_minute_trends(symbol, ndays.max(1)).await
    }

    async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        let secid = Exchange::infer_eastmoney_secid(stock_code);
        let url = "https://push2delay.eastmoney.com/api/qt/stock/get";
        let params = [
            ("secid", secid.as_str()),
            (
                "fields",
                "f57,f58,f11,f12,f13,f14,f15,f16,f17,f18,f19,f20,f31,f32,f33,f34,f35,f36,f37,f38,f39,f40",
            ),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
        ];

        debug!("Fetching order book for {}", stock_code);

        #[derive(Deserialize)]
        struct OrderBookResponse {
            data: Option<OrderBookFields>,
        }

        #[derive(Deserialize)]
        struct OrderBookFields {
            #[serde(rename = "f57", default)]
            code: String,
            #[serde(rename = "f58", default)]
            name: String,
            #[serde(rename = "f11", default)]
            f11: Option<i64>,
            #[serde(rename = "f12", default)]
            f12: Option<i64>,
            #[serde(rename = "f13", default)]
            f13: Option<i64>,
            #[serde(rename = "f14", default)]
            f14: Option<i64>,
            #[serde(rename = "f15", default)]
            f15: Option<i64>,
            #[serde(rename = "f16", default)]
            f16: Option<i64>,
            #[serde(rename = "f17", default)]
            f17: Option<i64>,
            #[serde(rename = "f18", default)]
            f18: Option<i64>,
            #[serde(rename = "f19", default)]
            f19: Option<i64>,
            #[serde(rename = "f20", default)]
            f20: Option<i64>,
            #[serde(rename = "f31", default)]
            f31: Option<i64>,
            #[serde(rename = "f32", default)]
            f32: Option<i64>,
            #[serde(rename = "f33", default)]
            f33: Option<i64>,
            #[serde(rename = "f34", default)]
            f34: Option<i64>,
            #[serde(rename = "f35", default)]
            f35: Option<i64>,
            #[serde(rename = "f36", default)]
            f36: Option<i64>,
            #[serde(rename = "f37", default)]
            f37: Option<i64>,
            #[serde(rename = "f38", default)]
            f38: Option<i64>,
            #[serde(rename = "f39", default)]
            f39: Option<i64>,
            #[serde(rename = "f40", default)]
            f40: Option<i64>,
        }

        let response: OrderBookResponse = self.request.get_json_with_params(url, &params).await?;
        let data = response
            .data
            .ok_or_else(|| crate::error::DataError::NoDataAvailable)?;

        Ok(parse_order_book_from_fields(
            &data.code,
            &data.name,
            [
                data.f11, data.f13, data.f15, data.f17, data.f19,
            ],
            [
                data.f12, data.f14, data.f16, data.f18, data.f20,
            ],
            [
                data.f31, data.f33, data.f35, data.f37, data.f39,
            ],
            [
                data.f32, data.f34, data.f36, data.f38, data.f40,
            ],
        ))
    }

    async fn get_ticks(&self, stock_code: &str) -> DataResult<Vec<TickData>> {
        let secid = Exchange::infer_eastmoney_secid(stock_code);
        let url = "https://push2delay.eastmoney.com/api/qt/stock/details/get";
        let params = [
            ("secid", secid.as_str()),
            ("fields1", "f1,f2,f3,f4"),
            ("fields2", "f51,f52,f53,f54,f55"),
            ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
            ("fltt", "2"),
            ("pos", "-11"),
        ];

        debug!("Fetching ticks for {}", stock_code);

        #[derive(Deserialize)]
        struct TickResponse {
            data: Option<TickResponseData>,
        }

        #[derive(Deserialize)]
        struct TickResponseData {
            #[serde(default)]
            details: Vec<String>,
        }

        let response: TickResponse = self.request.get_json_with_params(url, &params).await?;
        let details = response
            .data
            .map(|d| d.details)
            .unwrap_or_default();

        Ok(parse_tick_details(stock_code, &details))
    }
}

fn codes_equivalent(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    !left_trimmed.is_empty() && left_trimmed == right_trimmed
}

impl EastMoneySource {
    pub(crate) async fn fetch_current_quotes(
        &self,
        symbols: &[StockCode],
    ) -> DataResult<Vec<CurrentMarketData>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        const CHUNK_SIZE: usize = 40;
        let mut results = Vec::with_capacity(symbols.len());

        for chunk in symbols.chunks(CHUNK_SIZE) {
            let secids = chunk
                .iter()
                .map(|symbol| symbol.exchange.eastmoney_secid(&symbol.stock_code))
                .collect::<Vec<_>>()
                .join(",");

            let url = "https://push2delay.eastmoney.com/api/qt/ulist.np/get";
            let params = [
                ("fltt", "2"),
                ("secids", secids.as_str()),
                (
                    "fields",
                    "f2,f3,f4,f5,f6,f7,f12,f14,f15,f16,f17,f18",
                ),
                ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ];

            debug!("Fetching batch quotes for {} symbols", chunk.len());

            #[derive(Deserialize)]
            struct UlistResponse {
                data: Option<UlistData>,
            }

            #[derive(Deserialize)]
            struct UlistData {
                diff: Option<Vec<UlistRow>>,
            }

            #[derive(Deserialize)]
            struct UlistRow {
                #[serde(rename = "f12", default)]
                code: Option<String>,
                #[serde(rename = "f2", default)]
                price: Option<f64>,
                #[serde(rename = "f3", default)]
                change_pct: Option<f64>,
                #[serde(rename = "f4", default)]
                change: Option<f64>,
                #[serde(rename = "f5", default)]
                volume: Option<f64>,
                #[serde(rename = "f6", default)]
                amount: Option<f64>,
                #[serde(rename = "f14", default)]
                name: Option<String>,
                #[serde(rename = "f15", default)]
                high: Option<f64>,
                #[serde(rename = "f16", default)]
                low: Option<f64>,
                #[serde(rename = "f17", default)]
                open: Option<f64>,
                #[serde(rename = "f18", default)]
                pre_close: Option<f64>,
            }

            let response: UlistResponse = match self.request.get_json_with_params(url, &params).await {
                Ok(response) => response,
                Err(error) => {
                    warn!("Failed to fetch batch quotes: {}", error);
                    continue;
                }
            };

            let rows = response.data.and_then(|data| data.diff).unwrap_or_default();
            for symbol in chunk {
                let row = rows.iter().find(|row| {
                    row.code.as_deref().is_some_and(|code| {
                        codes_equivalent(code, &symbol.stock_code)
                    })
                });
                let Some(row) = row else {
                    continue;
                };
                let short_name = row
                    .name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| symbol.short_name.clone());
                results.push(CurrentMarketData {
                    stock_code: symbol.stock_code.clone(),
                    short_name,
                    price: row.price.unwrap_or(0.0),
                    open: row.open,
                    high: row.high,
                    low: row.low,
                    pre_close: row.pre_close,
                    change: row.change.unwrap_or(0.0),
                    change_pct: row.change_pct.unwrap_or(0.0),
                    volume: row.volume.unwrap_or(0.0) as u64,
                    amount: row.amount.unwrap_or(0.0),
                });
            }
        }

        if results.is_empty() {
            return Err(crate::error::DataError::NoDataAvailable);
        }

        Ok(results)
    }

    async fn fetch_stock_minute_trends(&self, symbol: &StockCode, ndays: u32) -> DataResult<Vec<MinuteData>> {
        let ndays_param = ndays.max(1).min(5);
        let use_history = ndays_param > 1 || matches!(symbol.exchange, Exchange::HK | Exchange::US);

        if use_history {
            match self.fetch_historical_minute_trends(symbol, ndays_param).await {
                Ok(data) if Self::minute_trading_days(&data) > 1 => return Ok(data),
                Ok(data) if ndays_param == 1 && !data.is_empty() => return Ok(data),
                Ok(_) | Err(_) => {
                    debug!(
                        "Historical minute trends unavailable for {}, falling back to realtime endpoint",
                        symbol.stock_code
                    );
                }
            }
        }

        let realtime_ndays = if ndays_param > 1 { 1 } else { ndays_param };
        self.fetch_realtime_minute_trends(symbol, realtime_ndays).await
    }

    async fn fetch_historical_minute_trends(
        &self,
        symbol: &StockCode,
        ndays: u32,
    ) -> DataResult<Vec<MinuteData>> {
        let secid = symbol.exchange.eastmoney_secid(&symbol.stock_code);
        let ndays = ndays.to_string();
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("ndays", ndays.as_str()),
            ("iscr", "0"),
            ("iscca", "0"),
            ("secid", secid.as_str()),
        ];

        debug!(
            "Fetching historical minute data from East Money: {} (ndays={})",
            symbol.stock_code, ndays
        );

        let response: MinuteResponse = self
            .history_request
            .get_json_with_params(
                "https://push2his.eastmoney.com/api/qt/stock/trends2/get",
                &params,
            )
            .await?;
        Self::parse_minute_response(symbol, response)
    }

    async fn fetch_realtime_minute_trends(
        &self,
        symbol: &StockCode,
        ndays: u32,
    ) -> DataResult<Vec<MinuteData>> {
        let secid = symbol.exchange.eastmoney_secid(&symbol.stock_code);
        let ndays = ndays.max(1).min(5).to_string();
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", ndays.as_str()),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", secid.as_str()),
        ];

        debug!(
            "Fetching realtime minute data from East Money: {} (ndays={})",
            symbol.stock_code, ndays
        );

        let response: MinuteResponse = self
            .request
            .get_json_with_params(
                "https://push2delay.eastmoney.com/api/qt/stock/trends2/get",
                &params,
            )
            .await?;
        Self::parse_minute_response(symbol, response)
    }

    fn parse_minute_response(
        symbol: &StockCode,
        response: MinuteResponse,
    ) -> DataResult<Vec<MinuteData>> {
        let data = match response.data {
            Some(data) => data,
            None => return Ok(Vec::new()),
        };

        let pre_close = data.pre_close;
        let trends = data.trends.unwrap_or_default();
        if trends.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(trends.len());
        let today = crate::util::cn_market_date(Utc::now());

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = parse_cn_market_time(time_str, today);

            let mut price: f64 = parts[2].parse().unwrap_or(0.0);
            if price <= 0.0 {
                price = parts[1].parse().unwrap_or(0.0);
            }
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[7].parse().unwrap_or(price);
            let price = if price <= 0.0 { avg_price } else { price };

            let change = price - pre_close;
            let change_pct = if pre_close > 0.0 {
                (change / pre_close * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };

            result.push(MinuteData {
                stock_code: symbol.stock_code.clone(),
                trade_time,
                price,
                change,
                change_pct,
                volume,
                avg_price,
                amount,
            });
        }

        result.sort_by_key(|m| m.trade_time);
        Ok(result)
    }

    fn minute_trading_days(data: &[MinuteData]) -> usize {
        let mut dates: Vec<_> = data
            .iter()
            .map(|minute| crate::util::cn_market_date(minute.trade_time))
            .collect();
        dates.sort();
        dates.dedup();
        dates.len()
    }
}

#[async_trait]
impl StockInfoSource for EastMoneySource {
    /// Fetches all available stock codes.
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        let url = super::CLIST_URL;
        let mut all_codes = Vec::new();
        let page_size = 100;
        let mut page = 1;

        loop {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f3".to_string()),
                (
                    "fs",
                    "m:0 t:6,m:0 t:80,m:1 t:2,m:1 t:23,m:0 t:81 s:2048".to_string(),
                ),
                ("fields", "f12,f14".to_string()),
            ];

            debug!("Fetching stock codes page {} from East Money", page);

            let response: StockListResponse =
                match self.request.get_json_with_params(url, &params).await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Failed to fetch page {}: {}", page, e);
                        break;
                    }
                };

            let items = match response.data.and_then(|d| d.diff) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            let count = items.len();

            for item in items {
                let exchange = Exchange::from_stock_code(&item.code);
                all_codes.push(StockCode {
                    stock_code: item.code,
                    short_name: item.name,
                    exchange,
                    list_date: None,
                });

                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }

            if count < page_size as usize {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }

    async fn search_stocks(&self, keyword: &str, limit: usize) -> DataResult<Vec<StockSearchHit>> {
        self.fetch_stock_search(keyword, limit).await
    }

    /// Fetches detailed stock information.
    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let exchange = Exchange::from_stock_code(stock_code);
        let secucode = format!("{}.{}", stock_code, exchange);

        #[derive(Deserialize)]
        struct OrgInfoItem {
            #[serde(rename = "ORG_NAME", default)]
            org_name: Option<String>,
            #[serde(rename = "SECURITY_NAME_ABBR", default)]
            short_name: Option<String>,
            #[serde(rename = "EM2016", default)]
            industry: Option<String>,
            #[serde(rename = "LISTING_DATE", default)]
            listing_date: Option<String>,
        }

        #[derive(Deserialize)]
        struct OrgInfoResult {
            data: Option<Vec<OrgInfoItem>>,
        }

        #[derive(Deserialize)]
        struct OrgInfoResponse {
            result: Option<OrgInfoResult>,
        }

        let url = "https://datacenter.eastmoney.com/securities/api/data/v1/get";
        let org_params = [
            ("reportName", "RPT_F10_ORG_BASICINFO"),
            ("columns", "ORG_NAME,SECURITY_NAME_ABBR,EM2016,LISTING_DATE"),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "1"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];

        let org_response: Result<OrgInfoResponse, _> =
            self.request.get_json_with_params(url, &org_params).await;

        let (full_name, short_name, industry, list_date) = match org_response {
            Ok(response) => response
                .result
                .and_then(|r| r.data)
                .and_then(|d| d.into_iter().next())
                .map(|item| {
                    let list_date = item.listing_date.and_then(|s| {
                        s.split(' ').next().and_then(|date_str| {
                            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
                        })
                    });
                    (
                        item.org_name.unwrap_or_default(),
                        item.short_name.unwrap_or_default(),
                        item.industry,
                        list_date,
                    )
                })
                .unwrap_or_default(),
            Err(_) => (String::new(), String::new(), None, None),
        };

        let shares_params = [
            ("reportName", "RPT_F10_EH_EQUITY"),
            ("columns", "TOTAL_SHARES,LISTED_A_SHARES"),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "1"),
            ("sortTypes", "-1"),
            ("sortColumns", "END_DATE"),
            ("source", "HSF10"),
            ("client", "PC"),
        ];

        let shares_response: Result<StockInfoResponse, _> =
            self.request.get_json_with_params(url, &shares_params).await;

        let (total_shares, circulating_shares) = match shares_response {
            Ok(response) => response
                .result
                .and_then(|r| r.data)
                .and_then(|d| d.first().cloned())
                .map(|item| {
                    (
                        item.total_shares.map(|v| v as u64),
                        item.listed_a_shares.map(|v| v as u64),
                    )
                })
                .unwrap_or((None, None)),
            Err(_) => (None, None),
        };

        Ok(StockInfo {
            stock_code: stock_code.to_string(),
            full_name,
            short_name,
            exchange,
            industry,
            total_shares,
            circulating_shares,
            list_date,
        })
    }
}

#[async_trait]
impl FundInfoSource for EastMoneySource {
    /// Fetches all available ETF codes.
    async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        let url = super::CLIST_URL;
        let mut all_codes = Vec::new();
        let page_size = 50;
        let mut page = 1;

        loop {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("ut", "bd1d9ddb04089700cf9c27f6f7426281".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("wbp2u", "|0|0|0|web".to_string()),
                ("fid", "f3".to_string()),
                ("fs", "b:MK0021,b:MK0022,b:MK0023,b:MK0024".to_string()),
                ("fields", "f12,f14,f2".to_string()),
            ];

            debug!("Fetching ETF codes page {} from East Money", page);

            let response: StockListResponse =
                match self.request.get_json_with_params(url, &params).await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Failed to fetch ETF page {}: {}", page, e);
                        break;
                    }
                };

            let items = match response.data.and_then(|d| d.diff) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            let count = items.len();

            for item in items {
                let exchange = Exchange::from_stock_code(&item.code);
                all_codes.push(ETFCode {
                    fund_code: item.code,
                    short_name: item.name,
                    exchange,
                    net_value: item.price,
                });

                if let Some(lim) = limit {
                    if all_codes.len() >= lim {
                        return Ok(all_codes);
                    }
                }
            }

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_codes)
    }
}

#[async_trait]
impl FundMarketSource for EastMoneySource {
    /// Fetches historical ETF K-line market data.
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let secid = Exchange::infer_eastmoney_secid(fund_code);
        let klt = k_type.to_api_value();

        let start = start_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| "19900101".to_string());
        let end = end_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116",
            ),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", &klt.to_string()),
            ("fqt", "1"),
            ("secid", &secid),
            ("beg", &start),
            ("end", &end),
        ];

        let klines = self.fetch_kline_lines(&params).await?;

        if klines.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(klines.len());

        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 11 {
                continue;
            }

            let trade_date = parse_trade_date(parts[0]);

            let open: f64 = parts[1].parse().unwrap_or(0.0);
            let close: f64 = parts[2].parse().unwrap_or(0.0);
            let high: f64 = parts[3].parse().unwrap_or(0.0);
            let low: f64 = parts[4].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let change_pct: f64 = parts[8].parse().unwrap_or(0.0);
            let change: f64 = parts[9].parse().unwrap_or(0.0);

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
                change: Some(change),
                change_pct: Some(change_pct),
            });
        }

        Ok(result)
    }

    /// Fetches real-time ETF quotes.
    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(fund_codes.len());

        for code in fund_codes {
            let secid = Exchange::infer_eastmoney_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2delay.eastmoney.com/api/qt/stock/get";
            debug!("Fetching ETF quote for {}", code);

            #[derive(Deserialize)]
            struct ETFGetResponse {
                data: Option<ETFGetData>,
            }

            #[derive(Deserialize)]
            struct ETFGetData {
                #[serde(rename = "f57")]
                code: String,
                #[serde(rename = "f58")]
                name: String,
                #[serde(rename = "f43", default)]
                price: Option<i64>,
                #[serde(rename = "f44", default)]
                high: Option<i64>,
                #[serde(rename = "f45", default)]
                low: Option<i64>,
                #[serde(rename = "f46", default)]
                open: Option<i64>,
                #[serde(rename = "f47", default)]
                volume: Option<u64>,
                #[serde(rename = "f48", default)]
                amount: Option<f64>,
                #[serde(rename = "f169", default)]
                change: Option<i64>,
                #[serde(rename = "f170", default)]
                change_pct: Option<i64>,
            }

            match self.request.get_json_with_params(url, &params).await {
                Ok(response) => {
                    let response: ETFGetResponse = response;
                    if let Some(data) = response.data {
                        results.push(ETFCurrentData {
                            fund_code: data.code,
                            short_name: data.name,
                            price: data.price.map(|v| v as f64 / 1000.0).unwrap_or(0.0),
                            open: data.open.map(|v| v as f64 / 1000.0),
                            high: data.high.map(|v| v as f64 / 1000.0),
                            low: data.low.map(|v| v as f64 / 1000.0),
                            change: data.change.map(|v| v as f64 / 1000.0),
                            change_pct: data.change_pct.map(|v| v as f64 / 100.0),
                            volume: data.volume.unwrap_or(0),
                            amount: data.amount.unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch ETF {}: {}", code, e);
                }
            }
        }

        Ok(results)
    }

    /// Fetches intraday ETF minute-level data.
    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let secid = Exchange::infer_eastmoney_secid(fund_code);

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", "1"),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", &secid),
        ];

        let url = "https://push2delay.eastmoney.com/api/qt/stock/trends2/get";
        debug!("Fetching ETF minute data from East Money: {}", fund_code);

        let response: MinuteResponse = self.request.get_json_with_params(url, &params).await?;

        let data = match response.data {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let pre_close = data.pre_close;
        let trends = data.trends.unwrap_or_default();

        if trends.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(trends.len());
        let today = crate::util::cn_market_date(Utc::now());

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = parse_cn_market_time(time_str, today);

            let price: f64 = parts[2].parse().unwrap_or(0.0);
            let volume: u64 = parts[5].parse::<f64>().unwrap_or(0.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[7].parse().unwrap_or(price);

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
