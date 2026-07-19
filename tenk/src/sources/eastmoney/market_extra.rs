use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use tracing::debug;

use super::EastMoneySource;
use crate::data::{
    BoardItem, CurrentMarketData, Exchange, IndexCode, KLineType, LimitPoolItem, LimitPoolKind,
    MacroRecord, MarketData, StockCode,
};
use crate::error::DataResult;
use crate::util::{
    eastmoney_secid_for_board, eastmoney_secid_for_hk, eastmoney_secid_for_index,
    eastmoney_secid_for_us, parse_trade_date,
};
use crate::traits::{
    BoardMarketSource, GlobalMarketSource, IndexMarketSource, LimitPoolSource, MacroSource,
};

impl EastMoneySource {
    async fn fetch_clist_boards(
        &self,
        fs: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<BoardItem>> {
        let url = "https://82.push2.eastmoney.com/api/qt/clist/get";
        let mut items = Vec::new();
        let page_size = 100;
        let mut page = 1u32;

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
                ("fs", fs.to_string()),
                ("fields", "f12,f14,f2,f3".to_string()),
            ];

            #[derive(Deserialize)]
            struct Resp {
                data: Option<RespData>,
            }
            #[derive(Deserialize)]
            struct RespData {
                diff: Option<Vec<Row>>,
            }
            #[derive(Deserialize)]
            struct Row {
                #[serde(rename = "f12")]
                code: String,
                #[serde(rename = "f14")]
                name: String,
                #[serde(rename = "f2", default)]
                price: Option<f64>,
                #[serde(rename = "f3", default)]
                change_pct: Option<f64>,
            }

            let response: Resp = self.request.get_json_with_params(url, &params).await?;
            let rows = response.data.and_then(|d| d.diff).unwrap_or_default();
            if rows.is_empty() {
                break;
            }
            let count = rows.len();
            for row in rows {
                items.push(BoardItem {
                    board_code: row.code,
                    board_name: row.name,
                    price: row.price.unwrap_or(0.0),
                    change_pct: row.change_pct.unwrap_or(0.0),
                });
                if let Some(lim) = limit {
                    if items.len() >= lim {
                        return Ok(items);
                    }
                }
            }
            if count < page_size as usize {
                break;
            }
            page += 1;
        }
        Ok(items)
    }

    async fn fetch_quote_batch(
        &self,
        secids: &[String],
    ) -> DataResult<Vec<CurrentMarketData>> {
        let mut results = Vec::with_capacity(secids.len());
        for secid in secids {
            let params = [
                ("secid", secid.as_str()),
                ("fields", "f43,f44,f45,f46,f47,f48,f57,f58,f60,f169,f170"),
                ("ut", "fa5fd1943c7b386f172d6893dbfba10b"),
            ];
            let url = "https://push2.eastmoney.com/api/qt/stock/get";
            #[derive(Deserialize)]
            struct Resp {
                data: Option<Row>,
            }
            #[derive(Deserialize)]
            struct Row {
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
            let response: Resp = self.request.get_json_with_params(url, &params).await?;
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
        Ok(results)
    }

    async fn fetch_kline(
        &self,
        secid: &str,
        label_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let klt = k_type.to_api_value();
        let start = start_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| "19900101".to_string());
        let end = end_date
            .map(|s| s.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116"),
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("klt", &klt.to_string()),
            ("fqt", "1"),
            ("secid", secid),
            ("beg", &start),
            ("end", &end),
        ];
        let url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
        #[derive(Deserialize)]
        struct KResp {
            data: Option<KData>,
        }
        #[derive(Deserialize)]
        struct KData {
            klines: Option<Vec<String>>,
        }
        let response: KResp = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();
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
                change,
                change_pct,
                turnover_ratio,
                pre_close: close - change,
            });
        }
        Ok(result)
    }

    fn index_exchange(code: &str) -> Exchange {
        if code.starts_with("399") {
            Exchange::SZ
        } else {
            Exchange::SH
        }
    }

    fn pool_endpoint(kind: LimitPoolKind) -> (&'static str, &'static str) {
        match kind {
            LimitPoolKind::LimitUp => ("getTopicZTPool", "fbt:asc"),
            LimitPoolKind::LimitDown => ("getTopicDTPool", "fund:asc"),
            LimitPoolKind::YesterdayLimitUp => ("getTopicZBPool", "fbt:asc"),
            LimitPoolKind::Strong => ("getTopicQSPool", "zsp:desc"),
            LimitPoolKind::SubNew => ("getTopicCXPool", "fbt:asc"),
            LimitPoolKind::BrokenBoard => ("getTopicBXPool", "fbt:asc"),
        }
    }
}

#[async_trait]
impl IndexMarketSource for EastMoneySource {
    async fn get_index_list(&self, limit: Option<usize>) -> DataResult<Vec<IndexCode>> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get";
        let mut items = Vec::new();
        let page_size = 100;
        let mut page = 1u32;

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
                ("fs", "m:1+t:1".to_string()),
                ("fields", "f12,f14".to_string()),
            ];
            #[derive(Deserialize)]
            struct Resp {
                data: Option<RespData>,
            }
            #[derive(Deserialize)]
            struct RespData {
                diff: Option<Vec<Row>>,
            }
            #[derive(Deserialize)]
            struct Row {
                #[serde(rename = "f12")]
                code: String,
                #[serde(rename = "f14")]
                name: String,
            }
            let response: Resp = self.request.get_json_with_params(url, &params).await?;
            let rows = response.data.and_then(|d| d.diff).unwrap_or_default();
            if rows.is_empty() {
                break;
            }
            let count = rows.len();
            for row in rows {
                let exchange = EastMoneySource::index_exchange(&row.code);
                items.push(IndexCode {
                    index_code: row.code,
                    index_name: row.name,
                    exchange,
                });
                if let Some(lim) = limit {
                    if items.len() >= lim {
                        return Ok(items);
                    }
                }
            }
            if count < page_size {
                break;
            }
            page += 1;
        }
        Ok(items)
    }

    async fn get_index_current(&self, index_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let secids: Vec<String> = index_codes
            .iter()
            .map(|code| {
                eastmoney_secid_for_index(code, EastMoneySource::index_exchange(code))
            })
            .collect();
        self.fetch_quote_batch(&secids).await
    }

    async fn get_index_market(
        &self,
        index_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let exchange = EastMoneySource::index_exchange(index_code);
        let secid = eastmoney_secid_for_index(index_code, exchange);
        self.fetch_kline(&secid, index_code, start_date, end_date, k_type)
            .await
    }
}

#[async_trait]
impl BoardMarketSource for EastMoneySource {
    async fn get_industry_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        self.fetch_clist_boards("m:90+t:2", limit).await
    }

    async fn get_concept_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        self.fetch_clist_boards("m:90+t:3", limit).await
    }

    async fn get_board_market(
        &self,
        board_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let secid = eastmoney_secid_for_board(board_code);
        self.fetch_kline(&secid, board_code, start_date, end_date, k_type)
            .await
    }

    async fn get_board_constituents(
        &self,
        board_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<StockCode>> {
        use crate::data::{Exchange, StockCode};

        let url = "https://29.push2.eastmoney.com/api/qt/clist/get";
        let mut items = Vec::new();
        let page_size = 100;
        let mut page = 1u32;
        let fs = format!("b:{board_code} f:!50");

        loop {
            let params = [
                ("pn", page.to_string()),
                ("pz", page_size.to_string()),
                ("po", "1".to_string()),
                ("np", "1".to_string()),
                ("fltt", "2".to_string()),
                ("invt", "2".to_string()),
                ("fid", "f12".to_string()),
                ("fs", fs.clone()),
                ("fields", "f12,f14".to_string()),
            ];
            #[derive(Deserialize)]
            struct Resp {
                data: Option<RespData>,
            }
            #[derive(Deserialize)]
            struct RespData {
                diff: Option<Vec<Row>>,
            }
            #[derive(Deserialize)]
            struct Row {
                #[serde(rename = "f12")]
                code: String,
                #[serde(rename = "f14")]
                name: String,
            }
            let response: Resp = self.request.get_json_with_params(url, &params).await?;
            let rows = response.data.and_then(|d| d.diff).unwrap_or_default();
            if rows.is_empty() {
                break;
            }
            let count = rows.len();
            for row in rows {
                let exchange = if row.code.starts_with('6') {
                    Exchange::SH
                } else if row.code.starts_with("920") {
                    Exchange::BJ
                } else {
                    Exchange::SZ
                };
                items.push(StockCode {
                    stock_code: row.code,
                    short_name: row.name,
                    exchange,
                    list_date: None,
                });
                if let Some(lim) = limit {
                    if items.len() >= lim {
                        return Ok(items);
                    }
                }
            }
            if count < page_size {
                break;
            }
            page += 1;
        }
        Ok(items)
    }
}

#[async_trait]
impl LimitPoolSource for EastMoneySource {
    async fn get_limit_pool(
        &self,
        kind: LimitPoolKind,
        date: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<LimitPoolItem>> {
        let (endpoint, sort) = Self::pool_endpoint(kind);
        let url = format!("https://push2ex.eastmoney.com/{endpoint}");
        let trade_date = date
            .map(|d| d.replace('-', ""))
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());
        let page_size = limit.unwrap_or(500).min(500);
        let params = [
            ("ut", "7eea3edcaed734bea9cbfc24409ed989"),
            ("dpt", "wz.ztzt"),
            ("Pageindex", "0"),
            ("pagesize", &page_size.to_string()),
            ("sort", sort),
            ("date", &trade_date),
        ];

        debug!("Fetching limit pool {:?} for {}", kind, trade_date);

        #[derive(Deserialize)]
        struct Resp {
            data: Option<PoolData>,
        }
        #[derive(Deserialize)]
        struct PoolData {
            pool: Option<Vec<PoolRow>>,
        }
        #[derive(Deserialize)]
        struct PoolRow {
            #[serde(rename = "c")]
            code: String,
            #[serde(rename = "n")]
            name: String,
            #[serde(rename = "p", default)]
            price: Option<f64>,
            #[serde(rename = "zdp", default)]
            change_pct: Option<f64>,
            #[serde(rename = "ztp", default)]
            limit_price: Option<f64>,
            #[serde(rename = "amount", default)]
            amount: Option<f64>,
            #[serde(rename = "hs", default)]
            turnover_ratio: Option<f64>,
            #[serde(rename = "ltsz", default)]
            float_cap: Option<f64>,
            #[serde(rename = "tshare", default)]
            total_cap: Option<f64>,
            #[serde(rename = "lbc", default)]
            boards: Option<u32>,
            #[serde(rename = "fbt", default)]
            first_time: Option<u64>,
            #[serde(rename = "lbt", default)]
            last_time: Option<u64>,
            #[serde(rename = "fund", default)]
            board_amount: Option<f64>,
            #[serde(rename = "hybk", default)]
            industry: Option<String>,
        }

        let response: Resp = self.request.get_json_with_params(&url, &params).await?;
        let rows = response.data.and_then(|d| d.pool).unwrap_or_default();
        let parsed_date = NaiveDate::parse_from_str(&trade_date, "%Y%m%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());

        Ok(rows
            .into_iter()
            .map(|row| LimitPoolItem {
                stock_code: row.code,
                stock_name: row.name,
                price: row.price.unwrap_or(0.0),
                change_pct: row.change_pct.unwrap_or(0.0),
                limit_price: row.limit_price.unwrap_or(0.0),
                amount: row.amount.unwrap_or(0.0),
                turnover_ratio: row.turnover_ratio.unwrap_or(0.0),
                float_market_cap: row.float_cap.unwrap_or(0.0),
                total_market_cap: row.total_cap.unwrap_or(0.0),
                continuous_boards: row.boards.unwrap_or(0),
                first_board_time: row.first_time.map(|v| v.to_string()).unwrap_or_default(),
                last_board_time: row.last_time.map(|v| v.to_string()).unwrap_or_default(),
                board_amount: row.board_amount.unwrap_or(0.0),
                industry: row.industry.unwrap_or_default(),
                trade_date: parsed_date,
            })
            .collect())
    }
}

#[async_trait]
impl MacroSource for EastMoneySource {
    async fn get_macro_cpi(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        self.fetch_macro("RPT_ECONOMY_CPI", "CPI", limit).await
    }

    async fn get_macro_gdp(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        self.fetch_macro("RPT_ECONOMY_GDP", "GDP", limit).await
    }
}

impl EastMoneySource {
    async fn fetch_macro(
        &self,
        report: &str,
        indicator: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<MacroRecord>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let page_size = limit.unwrap_or(50).min(500);
        let params = [
            ("reportName", report),
            ("columns", "ALL"),
            ("pageNumber", "1"),
            ("pageSize", &page_size.to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "REPORT_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];
        #[derive(Deserialize)]
        struct Resp {
            result: Option<RespResult>,
        }
        #[derive(Deserialize)]
        struct RespResult {
            data: Option<Vec<serde_json::Value>>,
        }
        let response: Resp = self.request.get_json_with_params(url, &params).await?;
        let rows = response.result.and_then(|r| r.data).unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let period = row.get("TIME")?.as_str()?.to_string();
                let report_date = row
                    .get("REPORT_DATE")?
                    .as_str()
                    .map(|s| parse_trade_date(&s[..10.min(s.len())]))
                    .unwrap_or_else(|| parse_trade_date(""));
                let mut values = Vec::new();
                if let Some(obj) = row.as_object() {
                    for (key, value) in obj {
                        if key == "REPORT_DATE" || key == "TIME" {
                            continue;
                        }
                        if let Some(num) = value.as_f64() {
                            values.push((key.clone(), num));
                        }
                    }
                }
                Some(MacroRecord {
                    indicator: indicator.to_string(),
                    period,
                    report_date,
                    values,
                })
            })
            .collect())
    }
}

#[async_trait]
impl GlobalMarketSource for EastMoneySource {
    async fn get_hk_current(&self, codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let secids: Vec<String> = codes.iter().map(|c| eastmoney_secid_for_hk(c)).collect();
        self.fetch_quote_batch(&secids).await
    }

    async fn get_us_current(&self, symbols: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let secids: Vec<String> = symbols.iter().map(|c| eastmoney_secid_for_us(c)).collect();
        self.fetch_quote_batch(&secids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_exchange() {
        assert_eq!(EastMoneySource::index_exchange("399001"), Exchange::SZ);
        assert_eq!(EastMoneySource::index_exchange("000001"), Exchange::SH);
    }

    #[test]
    fn test_pool_endpoint_mapping() {
        assert_eq!(EastMoneySource::pool_endpoint(LimitPoolKind::LimitUp).0, "getTopicZTPool");
        assert_eq!(
            EastMoneySource::pool_endpoint(LimitPoolKind::BrokenBoard).0,
            "getTopicBXPool"
        );
    }
}
