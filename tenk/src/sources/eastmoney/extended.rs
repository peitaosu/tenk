use super::*;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use tracing::debug;

use crate::data::{
    BillboardDetail, BillboardItem, BlockTradeData, CapitalFlowData,
    CapitalFlowHistory, DividendData, EarningsForecast, Exchange, FundHolding,
    IPOData, InstitutionalResearchData, MarginTradingData, ResearchReportData, StockConnectData, StockValuation, TopHolder,
};
use crate::error::DataResult;
use crate::util::parse_trade_date;
use crate::traits::{
    BillboardSource, BlockTradeSource, CapitalFlowSource,
    DividendSource, EarningsForecastSource,
    HoldingsSource, IPOSource, InstitutionalResearchSource, MarginTradingSource,
    ResearchReportSource, StockConnectSource, ValuationSource,
};

#[async_trait]
impl CapitalFlowSource for EastMoneySource {
    /// Fetches capital flow data for stocks.
    async fn get_capital_flow(&self, stock_codes: &[&str]) -> DataResult<Vec<CapitalFlowData>> {
        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        let secids: Vec<String> = stock_codes.iter().map(|c| Exchange::eastmoney_secid(c)).collect();
        let secids_str = secids.join(",");

        let url = "https://push2.eastmoney.com/api/qt/ulist.np/get";
        let params = [
            ("fltt", "2"),
            ("secids", &secids_str),
            (
                "fields",
                "f12,f14,f62,f184,f66,f69,f72,f75,f78,f81,f84,f87,f64,f65",
            ),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching capital flow for {} stocks", stock_codes.len());

        #[derive(Deserialize)]
        struct FlowResponse {
            data: Option<FlowData>,
        }

        #[derive(Deserialize)]
        struct FlowData {
            diff: Option<Vec<FlowItem>>,
        }

        #[derive(Deserialize)]
        struct FlowItem {
            #[serde(rename = "f12")]
            code: String,
            #[serde(rename = "f14")]
            name: String,
            #[serde(rename = "f62", default)]
            main_net: Option<f64>,
            #[serde(rename = "f64", default)]
            main_in: Option<f64>,
            #[serde(rename = "f65", default)]
            main_out: Option<f64>,
            #[serde(rename = "f66", default)]
            super_large_net: Option<f64>,
            #[serde(rename = "f72", default)]
            large_net: Option<f64>,
            #[serde(rename = "f78", default)]
            medium_net: Option<f64>,
            #[serde(rename = "f84", default)]
            small_net: Option<f64>,
            #[serde(rename = "f184", default)]
            main_net_ratio: Option<f64>,
        }

        let response: FlowResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.data.and_then(|d| d.diff).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| CapitalFlowData {
                stock_code: item.code,
                stock_name: item.name,
                main_net_inflow: item.main_net.unwrap_or(0.0),
                main_inflow: item.main_in.unwrap_or(0.0),
                main_outflow: item.main_out.unwrap_or(0.0),
                super_large_net_inflow: item.super_large_net.unwrap_or(0.0),
                large_net_inflow: item.large_net.unwrap_or(0.0),
                medium_net_inflow: item.medium_net.unwrap_or(0.0),
                small_net_inflow: item.small_net.unwrap_or(0.0),
                main_net_ratio: item.main_net_ratio.unwrap_or(0.0),
            })
            .collect())
    }

    /// Fetches historical capital flow data for a stock.
    async fn get_capital_flow_history(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<CapitalFlowHistory>> {
        let secid = Exchange::eastmoney_secid(stock_code);
        let lmt = limit.unwrap_or(30).to_string();

        let url = "https://push2his.eastmoney.com/api/qt/stock/fflow/daykline/get";
        let params = [
            ("secid", secid.as_str()),
            ("fields1", "f1,f2,f3,f7"),
            (
                "fields2",
                "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64,f65",
            ),
            ("lmt", &lmt),
            ("klt", "101"),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching capital flow history for {}", stock_code);

        #[derive(Deserialize)]
        struct HistoryResponse {
            data: Option<HistoryData>,
        }

        #[derive(Deserialize)]
        struct HistoryData {
            klines: Option<Vec<String>>,
        }

        let response: HistoryResponse = self.request.get_json_with_params(url, &params).await?;
        let klines = response.data.and_then(|d| d.klines).unwrap_or_default();

        let mut result = Vec::with_capacity(klines.len());
        for line in klines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 10 {
                continue;
            }

            let trade_date = parse_trade_date(parts[0]);

            result.push(CapitalFlowHistory {
                stock_code: stock_code.to_string(),
                trade_date,
                main_net_inflow: parts[1].parse().unwrap_or(0.0),
                small_net_inflow: parts[2].parse().unwrap_or(0.0),
                medium_net_inflow: parts[3].parse().unwrap_or(0.0),
                large_net_inflow: parts[4].parse().unwrap_or(0.0),
                super_large_net_inflow: parts[5].parse().unwrap_or(0.0),
                close: parts[8].parse().unwrap_or(0.0),
                change_pct: parts[9].parse().unwrap_or(0.0),
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl BillboardSource for EastMoneySource {
    /// Fetches Billboard list for a given date.
    async fn get_billboard_list(&self, date: Option<&str>) -> DataResult<Vec<BillboardItem>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let trade_date = date
            .map(|d| d.to_string())
            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
        let filter = format!(
            "(TRADE_DATE<='{trade_date}')(TRADE_DATE>='{trade_date}')"
        );

        debug!("Fetching Billboard list for {}", trade_date);

        #[derive(Deserialize)]
        struct DTResponse {
            result: Option<DTResult>,
        }

        #[derive(Deserialize)]
        struct DTResult {
            data: Option<Vec<DTItem>>,
        }

        #[derive(Deserialize)]
        struct DTItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "TRADE_DATE")]
            trade_date: String,
            #[serde(rename = "CLOSE_PRICE", default)]
            close: Option<f64>,
            #[serde(rename = "CHANGE_RATE", default)]
            change_rate: Option<f64>,
            #[serde(rename = "TURNOVERRATE", default)]
            turnover_rate: Option<f64>,
            #[serde(rename = "BILLBOARD_NET_AMT", default)]
            net_amt: Option<f64>,
            #[serde(rename = "BILLBOARD_BUY_AMT", default)]
            buy_amt: Option<f64>,
            #[serde(rename = "BILLBOARD_SELL_AMT", default)]
            sell_amt: Option<f64>,
            #[serde(rename = "EXPLANATION", default)]
            explanation: Option<String>,
        }

        let columns = "SECURITY_CODE,SECURITY_NAME_ABBR,TRADE_DATE,CLOSE_PRICE,CHANGE_RATE,TURNOVERRATE,BILLBOARD_NET_AMT,BILLBOARD_BUY_AMT,BILLBOARD_SELL_AMT,EXPLANATION";
        let mut all_items = Vec::new();
        let mut page_number = 1u32;

        loop {
            let page_str = page_number.to_string();
            let params = [
                ("reportName", "RPT_DAILYBILLBOARD_DETAILSNEW"),
                ("columns", columns),
                ("filter", &filter),
                ("pageNumber", &page_str),
                ("pageSize", "500"),
                ("sortTypes", "1,-1"),
                ("sortColumns", "SECURITY_CODE,TRADE_DATE"),
                ("source", "WEB"),
                ("client", "WEB"),
            ];

            let response: DTResponse = self.request.get_json_with_params(url, &params).await?;
            let items = response.result.and_then(|r| r.data).unwrap_or_default();
            if items.is_empty() {
                break;
            }
            let count = items.len();
            all_items.extend(items);
            if count < 500 {
                break;
            }
            page_number += 1;
        }

        Ok(all_items
            .into_iter()
            .map(|item| BillboardItem {
                stock_code: item.code,
                stock_name: item.name,
                trade_date: parse_trade_date(&item.trade_date[..10.min(item.trade_date.len())]),
                close: item.close.unwrap_or(0.0),
                change_pct: item.change_rate.unwrap_or(0.0),
                turnover_ratio: item.turnover_rate.unwrap_or(0.0),
                net_buy_amount: item.net_amt.unwrap_or(0.0),
                buy_amount: item.buy_amt.unwrap_or(0.0),
                sell_amount: item.sell_amt.unwrap_or(0.0),
                reason: item.explanation.unwrap_or_default(),
            })
            .collect())
    }

    /// Fetches Billboard detail for a given stock and date.
    async fn get_billboard_detail(
        &self,
        stock_code: &str,
        date: &str,
    ) -> DataResult<Vec<BillboardDetail>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let filter = format!("(TRADE_DATE='{}')(SECURITY_CODE=\"{}\")", date, stock_code);

        let buy_params = [
            ("reportName", "RPT_BILLBOARD_DAILYDETAILSBUY"),
            ("columns", "ALL"),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortTypes", "-1"),
            ("sortColumns", "BUY"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        let sell_params = [
            ("reportName", "RPT_BILLBOARD_DAILYDETAILSSELL"),
            ("columns", "ALL"),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortTypes", "-1"),
            ("sortColumns", "SELL"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!(
            "Fetching Dragon Tiger Detail for {} on {}",
            stock_code, date
        );

        #[derive(Deserialize)]
        struct DetailResponse {
            result: Option<DetailResult>,
        }

        #[derive(Deserialize)]
        struct DetailResult {
            data: Option<Vec<DetailItem>>,
        }

        #[derive(Deserialize)]
        struct DetailItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "TRADE_DATE")]
            trade_date: String,
            #[serde(rename = "OPERATEDEPT_NAME")]
            trader: String,
            #[serde(rename = "BUY", default)]
            buy: Option<f64>,
            #[serde(rename = "SELL", default)]
            sell: Option<f64>,
            #[serde(rename = "NET", default)]
            net: Option<f64>,
        }

        let mut results = Vec::new();

        let buy_response: DetailResponse =
            self.request.get_json_with_params(url, &buy_params).await?;
        if let Some(items) = buy_response.result.and_then(|r| r.data) {
            for item in items {
                let date = parse_trade_date(&item.trade_date[..10]);
                results.push(BillboardDetail {
                    stock_code: item.code,
                    trade_date: date,
                    trader_name: item.trader,
                    buy_amount: item.buy.unwrap_or(0.0),
                    sell_amount: item.sell.unwrap_or(0.0),
                    net_amount: item.net.unwrap_or(0.0),
                    direction: "buy".to_string(),
                });
            }
        }

        let sell_response: DetailResponse =
            self.request.get_json_with_params(url, &sell_params).await?;
        if let Some(items) = sell_response.result.and_then(|r| r.data) {
            for item in items {
                let date = parse_trade_date(&item.trade_date[..10]);
                results.push(BillboardDetail {
                    stock_code: item.code,
                    trade_date: date,
                    trader_name: item.trader,
                    buy_amount: item.buy.unwrap_or(0.0),
                    sell_amount: item.sell.unwrap_or(0.0),
                    net_amount: item.net.unwrap_or(0.0),
                    direction: "sell".to_string(),
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl EarningsForecastSource for EastMoneySource {
    /// Fetches earnings forecast for a given report period.
    async fn get_earnings_forecast(
        &self,
        report_period: Option<&str>,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<EarningsForecast>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let filter = report_period
            .map(|p| format!("(REPORT_DATE='{}')", p))
            .unwrap_or_default();

        let params = [
            ("reportName", "RPT_PUBLIC_OP_NEWPREDICT"),
            (
                "columns",
                "SECURITY_CODE,SECURITY_NAME_ABBR,PREDICT_FINANCE_CODE,PREDICT_TYPE,PREDICT_AMT_LOWER,PREDICT_AMT_UPPER,ADD_AMP_LOWER,ADD_AMP_UPPER,REPORT_DATE,NOTICE_DATE,CHANGE_REASON_EXPLAIN",
            ),
            ("filter", &filter),
            ("pageNumber", &page.to_string()),
            ("pageSize", &limit.to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "NOTICE_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching earnings forecast page {}", page);

        #[derive(Deserialize)]
        struct ForecastResponse {
            result: Option<ForecastResult>,
        }

        #[derive(Deserialize)]
        struct ForecastResult {
            data: Option<Vec<ForecastItem>>,
        }

        #[derive(Deserialize)]
        struct ForecastItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "PREDICT_TYPE", default)]
            predict_type: Option<String>,
            #[serde(rename = "PREDICT_AMT_LOWER", default)]
            profit_min: Option<f64>,
            #[serde(rename = "PREDICT_AMT_UPPER", default)]
            profit_max: Option<f64>,
            #[serde(rename = "ADD_AMP_LOWER", default)]
            change_min: Option<f64>,
            #[serde(rename = "ADD_AMP_UPPER", default)]
            change_max: Option<f64>,
            #[serde(rename = "REPORT_DATE", default)]
            report_date: Option<String>,
            #[serde(rename = "NOTICE_DATE")]
            notice_date: String,
            #[serde(rename = "CHANGE_REASON_EXPLAIN", default)]
            summary: Option<String>,
        }

        let response: ForecastResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let announce_date = parse_trade_date(&item.notice_date[..10]);
                EarningsForecast {
                    stock_code: item.code,
                    stock_name: item.name,
                    forecast_type: item.predict_type.unwrap_or_default(),
                    profit_min: item.profit_min,
                    profit_max: item.profit_max,
                    change_min: item.change_min,
                    change_max: item.change_max,
                    report_period: item.report_date.unwrap_or_default(),
                    announce_date,
                    summary: item.summary,
                }
            })
            .collect())
    }
}

#[async_trait]
impl StockConnectSource for EastMoneySource {
    /// Fetches Stock Connect data.
    async fn get_stock_connect(&self, limit: Option<usize>) -> DataResult<Vec<StockConnectData>> {
        let url = "https://push2his.eastmoney.com/api/qt/kamt.kline/get";
        let params = [
            ("fields1", "f1,f3,f5"),
            ("fields2", "f51,f52,f53,f54,f55,f56"),
            ("klt", "101"),
            ("lmt", &limit.unwrap_or(30).to_string()),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching Stock Connect data");

        #[derive(Deserialize)]
        struct ConnectResponse {
            data: Option<ConnectData>,
        }

        #[derive(Deserialize)]
        struct ConnectData {
            #[serde(default)]
            hk2sh: Vec<String>,
            #[serde(default)]
            hk2sz: Vec<String>,
        }

        let response: ConnectResponse = self.request.get_json_with_params(url, &params).await?;
        let data = match response.data {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let mut result = Vec::new();
        let sh_lines = &data.hk2sh;
        let sz_lines = &data.hk2sz;

        for i in 0..sh_lines.len().min(sz_lines.len()) {
            let sh_parts: Vec<&str> = sh_lines[i].split(',').collect();
            let sz_parts: Vec<&str> = sz_lines[i].split(',').collect();

            if sh_parts.len() < 4 || sz_parts.len() < 4 {
                continue;
            }

            let trade_date = parse_trade_date(sh_parts[0]);

            let sh_net: f64 = sh_parts[1].parse().unwrap_or(0.0);
            let sz_net: f64 = sz_parts[1].parse().unwrap_or(0.0);
            let sh_buy: f64 = sh_parts[2].parse().unwrap_or(0.0);
            let sh_sell: f64 = sh_parts[3].parse().unwrap_or(0.0);
            let sz_buy: f64 = sz_parts[2].parse().unwrap_or(0.0);
            let sz_sell: f64 = sz_parts[3].parse().unwrap_or(0.0);

            result.push(StockConnectData {
                trade_date,
                north_net_buy: sh_net + sz_net,
                sh_net_buy: sh_net,
                sz_net_buy: sz_net,
                north_buy: sh_buy + sz_buy,
                north_sell: sh_sell + sz_sell,
            });
        }

        Ok(result)
    }
}

#[async_trait]
impl MarginTradingSource for EastMoneySource {
    /// Fetches margin trading data for a stock.
    async fn get_margin_trading(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<MarginTradingData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";

        let params = [
            ("reportName", "RPTA_WEB_RZRQ_GGMX"),
            (
                "columns",
                "DATE,SCODE,SECNAME,RZYE,RZMRE,RZCHE,RQYE,RQYL,RZRQYE",
            ),
            ("filter", &format!("(SCODE=\"{}\")", stock_code)),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(30).to_string()),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching margin trading data for {}", stock_code);

        #[derive(Deserialize)]
        struct MarginResponse {
            result: Option<MarginResult>,
        }

        #[derive(Deserialize)]
        struct MarginResult {
            data: Option<Vec<MarginItem>>,
        }

        #[derive(Deserialize)]
        struct MarginItem {
            #[serde(rename = "SCODE")]
            code: String,
            #[serde(rename = "SECNAME")]
            name: String,
            #[serde(rename = "DATE")]
            trade_date: String,
            #[serde(rename = "RZYE", default)]
            margin_balance: Option<f64>,
            #[serde(rename = "RZMRE", default)]
            margin_buy: Option<f64>,
            #[serde(rename = "RZCHE", default)]
            margin_repay: Option<f64>,
            #[serde(rename = "RQYE", default)]
            short_balance: Option<f64>,
            #[serde(rename = "RQYL", default)]
            short_volume: Option<f64>,
            #[serde(rename = "RZRQYE", default)]
            total_balance: Option<f64>,
        }

        let response: MarginResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let trade_date = parse_trade_date(&item.trade_date[..10]);
                MarginTradingData {
                    stock_code: item.code,
                    stock_name: item.name,
                    trade_date,
                    margin_balance: item.margin_balance.unwrap_or(0.0),
                    margin_buy: item.margin_buy.unwrap_or(0.0),
                    margin_repay: item.margin_repay.unwrap_or(0.0),
                    short_balance: item.short_balance.unwrap_or(0.0),
                    short_volume: item.short_volume.unwrap_or(0.0) as u64,
                    total_balance: item.total_balance.unwrap_or(0.0),
                }
            })
            .collect())
    }
}

#[async_trait]
impl IPOSource for EastMoneySource {
    /// Fetches IPO list.
    async fn get_ipo_list(&self, limit: Option<usize>) -> DataResult<Vec<IPOData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let params = [
            ("reportName", "RPTA_APP_IPOAPPLY"),
            (
                "columns",
                "SECURITY_CODE,SECURITY_NAME,ISSUE_PRICE,APPLY_DATE,LISTING_DATE,ONLINE_ISSUE_LWR,TOTAL_ISSUE_NUM,ONLINE_ISSUE_NUM,INDUSTRY_PE_NEW",
            ),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "APPLY_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching IPO list");

        #[derive(Deserialize)]
        struct IPOResponse {
            result: Option<IPOResult>,
        }

        #[derive(Deserialize)]
        struct IPOResult {
            data: Option<Vec<IPOItem>>,
        }

        #[derive(Deserialize)]
        struct IPOItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME")]
            name: String,
            #[serde(rename = "ISSUE_PRICE", default)]
            issue_price: Option<f64>,
            #[serde(rename = "APPLY_DATE")]
            apply_date: String,
            #[serde(rename = "LISTING_DATE", default)]
            list_date: Option<String>,
            #[serde(rename = "ONLINE_ISSUE_LWR", default)]
            winning_rate: Option<f64>,
            #[serde(rename = "TOTAL_ISSUE_NUM", default)]
            issue_quantity: Option<f64>,
            #[serde(rename = "ONLINE_ISSUE_NUM", default)]
            online_quantity: Option<f64>,
            #[serde(rename = "INDUSTRY_PE_NEW", default)]
            pe_ratio: Option<f64>,
        }

        let response: IPOResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let sub_date = parse_trade_date(&item.apply_date[..10]);
                let list_date = item
                    .list_date
                    .as_ref()
                    .and_then(|d| NaiveDate::parse_from_str(&d[..10], "%Y-%m-%d").ok());
                IPOData {
                    stock_code: item.code,
                    stock_name: item.name,
                    issue_price: item.issue_price.unwrap_or(0.0),
                    sub_date,
                    list_date,
                    winning_rate: item.winning_rate,
                    issue_quantity: item.issue_quantity.map(|v| (v * 10000.0) as u64),
                    online_quantity: item.online_quantity.map(|v| v as u64),
                    pe_ratio: item.pe_ratio,
                }
            })
            .collect())
    }
}

#[async_trait]
impl BlockTradeSource for EastMoneySource {
    /// Fetches block trade list with buyer/seller info.
    async fn get_block_trades(&self, limit: Option<usize>) -> DataResult<Vec<BlockTradeData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let page_size = limit.unwrap_or(50).to_string();
        let params = [
            ("reportName", "RPT_DATA_BLOCKTRADE"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,TRADE_DATE,CLOSE_PRICE,DEAL_PRICE,PREMIUM_RATIO,DEAL_AMT,BUYER_NAME,SELLER_NAME",
            ),
            ("pageNumber", "1"),
            ("pageSize", &page_size),
            ("sortTypes", "-1"),
            ("sortColumns", "TRADE_DATE"),
        ];

        debug!("Fetching block trade list");

        #[derive(Deserialize)]
        struct BlockResponse {
            result: Option<BlockResult>,
        }

        #[derive(Deserialize)]
        struct BlockResult {
            data: Option<Vec<BlockItem>>,
        }

        #[derive(Deserialize)]
        struct BlockItem {
            #[serde(rename = "SECUCODE")]
            secucode: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "TRADE_DATE")]
            trade_date: String,
            #[serde(rename = "DEAL_PRICE", default)]
            price: Option<f64>,
            #[serde(rename = "CLOSE_PRICE", default)]
            close_price: Option<f64>,
            #[serde(rename = "PREMIUM_RATIO", default)]
            premium_rate: Option<f64>,
            #[serde(rename = "DEAL_AMT", default)]
            amount: Option<f64>,
            #[serde(rename = "BUYER_NAME", default)]
            buyer: Option<String>,
            #[serde(rename = "SELLER_NAME", default)]
            seller: Option<String>,
        }

        let response: BlockResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let trade_date = parse_trade_date(&item.trade_date[..10]);
                let stock_code = item
                    .secucode
                    .split('.')
                    .next()
                    .unwrap_or(&item.secucode)
                    .to_string();
                BlockTradeData {
                    stock_code,
                    stock_name: item.name,
                    trade_date,
                    price: item.price.unwrap_or(0.0),
                    close_price: item.close_price.unwrap_or(0.0),
                    premium_rate: item.premium_rate.unwrap_or(0.0) * 100.0,
                    volume: 0,
                    amount: item.amount.unwrap_or(0.0),
                    buyer: item.buyer.unwrap_or_default(),
                    seller: item.seller.unwrap_or_default(),
                }
            })
            .collect())
    }
}

#[async_trait]
impl InstitutionalResearchSource for EastMoneySource {
    /// Fetches institutional research list.
    async fn get_institutional_research(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<InstitutionalResearchData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let params = [
            ("reportName", "RPT_ORG_SURVEYNEW"),
            (
                "columns",
                "SECUCODE,SECURITY_CODE,SECURITY_NAME_ABBR,NOTICE_DATE,NUMBERNEW,RECEIVE_OBJECT,RECEIVE_WAY_EXPLAIN,RECEPTIONIST",
            ),
            ("pageNumber", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("sortTypes", "-1"),
            ("sortColumns", "NOTICE_DATE"),
            ("source", "WEB"),
            ("client", "WEB"),
        ];

        debug!("Fetching institutional research list");

        #[derive(Deserialize)]
        struct ResearchResponse {
            result: Option<ResearchResult>,
        }

        #[derive(Deserialize)]
        struct ResearchResult {
            data: Option<Vec<ResearchItem>>,
        }

        #[derive(Deserialize)]
        struct ResearchItem {
            #[serde(rename = "SECURITY_CODE")]
            code: String,
            #[serde(rename = "SECURITY_NAME_ABBR")]
            name: String,
            #[serde(rename = "NOTICE_DATE")]
            notice_date: String,
            #[serde(rename = "NUMBERNEW", default)]
            org_num: Option<String>,
            #[serde(rename = "RECEIVE_OBJECT", default)]
            org_name: Option<String>,
            #[serde(rename = "RECEIVE_WAY_EXPLAIN", default)]
            receive_way: Option<String>,
            #[serde(rename = "RECEPTIONIST", default)]
            receptionist: Option<String>,
        }

        let response: ResearchResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let research_date = parse_trade_date(&item.notice_date[..10]);
                InstitutionalResearchData {
                    stock_code: item.code,
                    stock_name: item.name,
                    research_date,
                    institution_count: item.org_num.and_then(|s| s.parse().ok()).unwrap_or(0),
                    institutions: item.org_name.unwrap_or_default(),
                    research_type: item.receive_way.unwrap_or_default(),
                    researchers: item.receptionist,
                }
            })
            .collect())
    }
}

#[async_trait]
impl ResearchReportSource for EastMoneySource {
    /// Fetches research reports.
    async fn get_research_reports(
        &self,
        stock_code: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<ResearchReportData>> {
        let url = "https://reportapi.eastmoney.com/report/list";
        let code = stock_code.unwrap_or("*");
        let params = [
            ("industryCode", code),
            ("pageNo", "1"),
            ("pageSize", &limit.unwrap_or(50).to_string()),
            ("qType", "0"),
            ("beginTime", "2020-01-01"),
            ("endTime", "2030-12-31"),
            ("sortColumn", "publishDate"),
            ("sortType", "-1"),
        ];

        debug!("Fetching research reports");

        #[derive(Deserialize)]
        struct ReportResponse {
            data: Option<Vec<ReportItem>>,
        }

        #[derive(Deserialize)]
        struct ReportItem {
            #[serde(rename = "infoCode")]
            info_code: String,
            #[serde(rename = "stockCode", default)]
            stock_code: Option<String>,
            #[serde(rename = "stockName", default)]
            stock_name: Option<String>,
            #[serde(default)]
            title: String,
            #[serde(rename = "orgSName", default)]
            org_name: Option<String>,
            #[serde(rename = "researcher", default)]
            researcher: Option<String>,
            #[serde(rename = "emRatingName", default)]
            rating: Option<String>,
            #[serde(rename = "publishDate")]
            publish_date: String,
        }

        let response: ReportResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.data.unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let publish_date = parse_trade_date(&item.publish_date[..10]);
                ResearchReportData {
                    report_id: item.info_code,
                    stock_code: item.stock_code.unwrap_or_default(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    title: item.title,
                    institution: item.org_name.unwrap_or_default(),
                    analysts: item.researcher.unwrap_or_default(),
                    rating: item.rating,
                    publish_date,
                }
            })
            .collect())
    }
}

#[async_trait]
impl ValuationSource for EastMoneySource {
    async fn get_valuation(&self, stock_code: &str) -> DataResult<StockValuation> {
        let secid = Exchange::eastmoney_secid(stock_code);

        let url = "https://push2.eastmoney.com/api/qt/stock/get";
        let params = [
            ("secid", secid.as_str()),
            (
                "fields",
                "f43,f57,f58,f116,f117,f162,f163,f167,f168,f164,f165,f166,f173,f183,f184,f185,f186,f187",
            ),
            ("ut", "b2884a393a59ad64002292a3e90d46a5"),
        ];

        debug!("Fetching valuation for {}", stock_code);

        #[derive(Deserialize)]
        struct ValResponse {
            data: Option<ValData>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ValData {
            f43: Option<f64>,
            f57: Option<String>,
            f58: Option<String>,
            f116: Option<f64>,
            f117: Option<f64>,
            f162: Option<f64>,
            f163: Option<f64>,
            f167: Option<f64>,
            f168: Option<f64>,
            f164: Option<f64>,
            f165: Option<f64>,
            f166: Option<f64>,
            f173: Option<f64>,
            f183: Option<f64>,
            f184: Option<f64>,
            f185: Option<f64>,
            f186: Option<f64>,
            f187: Option<f64>,
        }

        let response: ValResponse = self.request.get_json_with_params(url, &params).await?;
        let data = response
            .data
            .ok_or_else(|| crate::error::DataError::NoDataAvailable)?;

        Ok(StockValuation {
            stock_code: data.f57.unwrap_or_else(|| stock_code.to_string()),
            stock_name: data.f58.unwrap_or_default(),
            price: data.f43.map(|p| p / 100.0).unwrap_or(0.0),
            market_cap: data.f116.unwrap_or(0.0),
            float_cap: data.f117.unwrap_or(0.0),
            pe_ttm: data.f162.map(|p| p / 100.0),
            pe_static: data.f163.map(|p| p / 100.0),
            pb: data.f167.map(|p| p / 100.0),
            ps: data.f168.map(|p| p / 100.0),
            eps: data.f173,
            bps: data.f187,
            roe: data.f164.map(|p| p / 100.0),
            gross_margin: data.f165.map(|p| p / 100.0),
            net_margin: data.f166.map(|p| p / 100.0),
            revenue: data.f183,
            net_profit: data.f184,
            revenue_yoy: data.f185,
            profit_yoy: data.f186,
        })
    }
}

#[async_trait]
impl HoldingsSource for EastMoneySource {
    async fn get_top_holders(&self, stock_code: &str) -> DataResult<Vec<TopHolder>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let filter = format!("(SECUCODE=\"{}\")", secucode);
        let params = [
            ("reportName", "RPT_DMSK_HOLDERS"),
            (
                "columns",
                "SECUCODE,END_DATE,HOLDER_NAME,HOLD_NUM,HOLD_RATIO",
            ),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", "10"),
            ("sortColumns", "END_DATE,HOLD_RATIO"),
            ("sortTypes", "-1,-1"),
        ];

        debug!("Fetching top holders for {}", stock_code);

        #[derive(Deserialize)]
        struct HolderResponse {
            result: Option<HolderResult>,
        }

        #[derive(Deserialize)]
        struct HolderResult {
            data: Option<Vec<HolderItem>>,
        }

        #[derive(Deserialize)]
        struct HolderItem {
            #[serde(rename = "END_DATE")]
            end_date: Option<String>,
            #[serde(rename = "HOLDER_NAME")]
            holder_name: Option<String>,
            #[serde(rename = "HOLD_NUM")]
            hold_num: Option<f64>,
            #[serde(rename = "HOLD_RATIO")]
            hold_ratio: Option<f64>,
        }

        let response: HolderResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let report_date = item
                    .end_date
                    .as_ref()
                    .map(|d| parse_trade_date(&d[..10]))
                    .unwrap_or_else(|| parse_trade_date(""));
                TopHolder {
                    stock_code: stock_code.to_string(),
                    report_date,
                    rank: (i + 1) as u32,
                    holder_name: item.holder_name.unwrap_or_default(),
                    hold_quantity: item.hold_num.unwrap_or(0.0) as u64,
                    hold_ratio: item.hold_ratio.unwrap_or(0.0),
                    change_quantity: None,
                    holder_type: String::new(),
                }
            })
            .collect())
    }

    async fn get_fund_holdings(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<FundHolding>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let page_size = limit.unwrap_or(20).to_string();
        let filter = format!("(SECUCODE=\"{}\")(HOLDER_TYPE=\"证券投资基金\")", secucode);
        let params = [
            ("reportName", "RPT_F10_EH_FREEHOLDERS"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,END_DATE,HOLDER_NAME,HOLD_NUM,HOLD_RATIO",
            ),
            ("filter", &filter),
            ("pageNumber", "1"),
            ("pageSize", &page_size),
            ("sortColumns", "END_DATE,HOLD_NUM"),
            ("sortTypes", "-1,-1"),
        ];

        debug!("Fetching fund holdings for {}", stock_code);

        #[derive(Deserialize)]
        struct FundResponse {
            result: Option<FundResult>,
        }

        #[derive(Deserialize)]
        struct FundResult {
            data: Option<Vec<FundItem>>,
        }

        #[derive(Deserialize)]
        struct FundItem {
            #[serde(rename = "SECURITY_NAME_ABBR")]
            stock_name: Option<String>,
            #[serde(rename = "END_DATE")]
            end_date: Option<String>,
            #[serde(rename = "HOLDER_NAME")]
            holder_name: Option<String>,
            #[serde(rename = "HOLD_NUM")]
            hold_num: Option<f64>,
            #[serde(rename = "HOLD_RATIO")]
            hold_ratio: Option<f64>,
        }

        let response: FundResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let report_date = item
                    .end_date
                    .as_ref()
                    .map(|d| parse_trade_date(&d[..10]))
                    .unwrap_or_else(|| parse_trade_date(""));
                FundHolding {
                    stock_code: stock_code.to_string(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    report_date,
                    fund_name: item.holder_name.unwrap_or_default(),
                    hold_shares: item.hold_num.unwrap_or(0.0) as u64,
                    hold_ratio: item.hold_ratio.unwrap_or(0.0),
                }
            })
            .collect())
    }
}

#[async_trait]
impl DividendSource for EastMoneySource {
    async fn get_dividends(&self, stock_code: &str) -> DataResult<Vec<DividendData>> {
        let url = "https://datacenter-web.eastmoney.com/api/data/v1/get";
        let secucode = format!(
            "{}.{}",
            stock_code,
            if stock_code.starts_with('6') {
                "SH"
            } else {
                "SZ"
            }
        );
        let params = [
            ("reportName", "RPT_SHAREBONUS_DET"),
            (
                "columns",
                "SECUCODE,SECURITY_NAME_ABBR,REPORT_DATE,EX_DIVIDEND_DATE,EQUITY_RECORD_DATE,PRETAX_BONUS_RMB,BONUS_RATIO,IT_RATIO",
            ),
            ("filter", &format!("(SECUCODE=\"{}\")", secucode)),
            ("pageNumber", "1"),
            ("pageSize", "50"),
            ("sortColumns", "REPORT_DATE"),
            ("sortTypes", "-1"),
        ];

        debug!("Fetching dividends for {}", stock_code);

        #[derive(Deserialize)]
        struct DivResponse {
            result: Option<DivResult>,
        }

        #[derive(Deserialize)]
        struct DivResult {
            data: Option<Vec<DivItem>>,
        }

        #[derive(Deserialize)]
        struct DivItem {
            #[serde(rename = "SECURITY_NAME_ABBR")]
            stock_name: Option<String>,
            #[serde(rename = "REPORT_DATE")]
            report_date: Option<String>,
            #[serde(rename = "EX_DIVIDEND_DATE")]
            ex_date: Option<String>,
            #[serde(rename = "EQUITY_RECORD_DATE")]
            record_date: Option<String>,
            #[serde(rename = "PRETAX_BONUS_RMB")]
            cash_div: Option<f64>,
            #[serde(rename = "BONUS_RATIO")]
            bonus: Option<f64>,
            #[serde(rename = "IT_RATIO")]
            transfer: Option<f64>,
        }

        let response: DivResponse = self.request.get_json_with_params(url, &params).await?;
        let items = response.result.and_then(|r| r.data).unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let report_date = item
                    .report_date
                    .as_ref()
                    .map(|d| parse_trade_date(&d[..10]))
                    .unwrap_or_else(|| parse_trade_date(""));
                let ex_date = item
                    .ex_date
                    .as_ref()
                    .map(|d| parse_trade_date(&d[..10]));
                let record_date = item
                    .record_date
                    .as_ref()
                    .map(|d| parse_trade_date(&d[..10]));
                DividendData {
                    stock_code: stock_code.to_string(),
                    stock_name: item.stock_name.unwrap_or_default(),
                    report_date,
                    ex_date,
                    record_date,
                    dividend_per_share: item.cash_div.unwrap_or(0.0),
                    bonus_shares: item.bonus.unwrap_or(0.0),
                    transfer_shares: item.transfer.unwrap_or(0.0),
                    dividend_yield: None,
                }
            })
            .collect())
    }
}
