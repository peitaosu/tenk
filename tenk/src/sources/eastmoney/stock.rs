use super::*;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::data::{
    CurrentMarketData, ETFCode,
    ETFCurrentData, ETFMarketData, ETFMinuteData, Exchange, KLineType, MarketData, MinuteData, OrderBookData, StockCode, StockInfo, TickData,
};
use crate::error::DataResult;
use crate::util::{parse_order_book_from_fields, parse_tick_details, parse_trade_date};
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
            .get("https://push2.eastmoney.com/api/qt/stock/trends2/get")
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
        let secid = Exchange::eastmoney_secid(stock_code);
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

        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        debug!("Fetching market data from East Money: {}", stock_code);

        let response: KLineResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

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
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(stock_codes.len());

        for code in stock_codes {
            let secid = Exchange::eastmoney_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            debug!("Fetching stock quote for {}", code);

            #[derive(Deserialize)]
            struct StockGetResponse {
                data: Option<StockGetData>,
            }

            #[derive(Deserialize)]
            struct StockGetData {
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
                #[serde(rename = "f60", default)]
                pre_close: Option<i64>,
                #[serde(rename = "f169", default)]
                change: Option<i64>,
                #[serde(rename = "f170", default)]
                change_pct: Option<i64>,
            }

            match self.request.get_json_with_params(url, &params).await {
                Ok(response) => {
                    let response: StockGetResponse = response;
                    if let Some(data) = response.data {
                        results.push(CurrentMarketData {
                            stock_code: data.code,
                            short_name: data.name,
                            price: data.price.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            open: data.open.map(|v| v as f64 / 100.0),
                            high: data.high.map(|v| v as f64 / 100.0),
                            low: data.low.map(|v| v as f64 / 100.0),
                            pre_close: data.pre_close.map(|v| v as f64 / 100.0),
                            change: data.change.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            change_pct: data.change_pct.map(|v| v as f64 / 100.0).unwrap_or(0.0),
                            volume: data.volume.unwrap_or(0) * 100,
                            amount: data.amount.unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch stock {}: {}", code, e);
                }
            }
        }

        Ok(results)
    }

    /// Fetches intraday minute-level data.
    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let secid = Exchange::eastmoney_secid(stock_code);

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", "1"),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", &secid),
        ];

        let url = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
        debug!("Fetching minute data from East Money: {}", stock_code);

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
        let today = Utc::now().date_naive();

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = if time_str.contains(' ') {
                chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M")
                    .map(|dt| Utc.from_utc_datetime(&dt))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                    .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                    .unwrap_or_else(|_| Utc::now())
            };

            let price: f64 = parts[2].parse().unwrap_or(0.0);
            let volume: u64 = (parts[5].parse::<f64>().unwrap_or(0.0) * 100.0) as u64;
            let amount: f64 = parts[6].parse().unwrap_or(0.0);
            let avg_price: f64 = parts[7].parse().unwrap_or(price);

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

    async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        let secid = Exchange::eastmoney_secid(stock_code);
        let url = "https://push2.eastmoney.com/api/qt/stock/get";
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
        let secid = Exchange::eastmoney_secid(stock_code);
        let url = "https://push2.eastmoney.com/api/qt/stock/details/get";
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

#[async_trait]
impl StockInfoSource for EastMoneySource {
    /// Fetches all available stock codes.
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
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
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
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
        let secid = Exchange::eastmoney_secid(fund_code);
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

        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        debug!("Fetching ETF market data from East Money: {}", fund_code);

        let response: KLineResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

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
            let secid = Exchange::eastmoney_secid(code);
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
            ];

            let url = "https://push2.eastmoney.com/api/qt/stock/get";
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
        let secid = Exchange::eastmoney_secid(fund_code);

        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ("ndays", "1"),
            ("iscr", "1"),
            ("iscca", "0"),
            ("secid", &secid),
        ];

        let url = "https://push2.eastmoney.com/api/qt/stock/trends2/get";
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
        let today = Utc::now().date_naive();

        for line in trends {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 8 {
                continue;
            }

            let time_str = parts[0];
            let trade_time = if time_str.contains(' ') {
                chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M")
                    .map(|dt| Utc.from_utc_datetime(&dt))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                    .map(|t| Utc.from_utc_datetime(&today.and_time(t)))
                    .unwrap_or_else(|_| Utc::now())
            };

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
