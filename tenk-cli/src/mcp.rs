//! MCP Server for tenk.

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use tenk::{DataClient, KLineType, NewsCategory, RelatedStock, format_related_stocks};

use crate::args::{
    BoardKindArg, FinancialKindArg, OptionExchangeArg, PoolKindArg, limit_kline,
};

/// MCP server implementation.
#[derive(Clone)]
pub struct TenkMCPServer {
    client: Arc<DataClient>,
}

impl TenkMCPServer {
    pub fn new() -> Self {
        let proxy = std::env::var("TENK_PROXY").ok();
        Self {
            client: Arc::new(
                crate::client::default_client(proxy.as_deref())
                    .expect("failed to build MCP client"),
            ),
        }
    }


    fn to_json<T: Serialize>(value: &T) -> Result<String, McpError> {
        serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn ok(text: String) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    async fn json_result<T>(
        future: impl std::future::Future<Output = tenk::DataResult<T>>,
    ) -> Result<CallToolResult, McpError>
    where
        T: Serialize,
    {
        let data = future
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Self::ok(Self::to_json(&data)?)
    }
}

/// Parameters for multiple symbols.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolsParam {
    /// List of symbols
    pub symbols: Vec<String>,
}

/// Parameter for a single symbol.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolParam {
    /// Symbol code
    pub symbol: String,
}

/// Parameters for K-line data.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KlineParams {
    /// Symbol code
    pub symbol: String,

    /// K-line type
    #[serde(default = "default_kline_type")]
    pub kline_type: String,

    /// Start date
    pub start: Option<String>,

    /// End date
    pub end: Option<String>,

    /// Maximum number of records
    pub limit: Option<usize>,
}

/// Parameters for tick data.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TicksParams {
    /// Symbol code
    pub symbol: String,

    /// Maximum number of records
    #[serde(default = "default_ticks_limit")]
    pub limit: usize,
}

/// Parameters for listing data.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    /// Exchange filter
    pub exchange: Option<String>,

    /// Maximum number of records
    pub limit: Option<usize>,
}

/// Parameters for bond quotes.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BondQuoteParams {
    /// List of bond symbols
    #[serde(default)]
    pub symbols: Vec<String>,

    /// Top N gainers
    pub top_gainers: Option<usize>,

    /// Top N losers
    pub top_losers: Option<usize>,

    /// Top N by volume
    pub top_volume: Option<usize>,
}

/// Parameter for limit.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LimitParam {
    /// Maximum number of records to return
    pub limit: Option<usize>,
}

/// Parameters for news list.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewsListParams {
    /// News category
    #[serde(default = "default_news_category")]
    pub category: String,
    /// Page number
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of articles
    #[serde(default = "default_news_limit")]
    pub limit: u32,
}

/// Parameter for news ID.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewsIdParam {
    /// News ID
    pub id: String,
}

/// Parameters for capital flow.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CapitalFlowParams {
    /// List of stock symbols
    pub symbols: Vec<String>,
}

/// Parameters for capital flow history.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CapitalFlowHistoryParams {
    /// Stock symbol
    pub symbol: String,
    /// Number of days
    pub limit: Option<usize>,
}

/// Parameters for Billboard list.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BillboardParams {
    /// Trade date
    pub date: Option<String>,
}

/// Parameters for Billboard detail.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BillboardDetailParams {
    /// Stock symbol
    pub symbol: String,
    /// Trade date
    pub date: String,
}

/// Parameters for earnings forecast.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EarningsForecastParams {
    /// Report period
    pub report_period: Option<String>,
    /// Page number
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of records
    #[serde(default = "default_forecast_limit")]
    pub limit: u32,
}

/// Parameters for Stock Connect.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StockConnectParams {
    /// Number of days
    pub limit: Option<usize>,
}

/// Parameters for margin trading.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarginTradingParams {
    /// Stock symbol
    pub symbol: String,
    /// Number of days
    pub limit: Option<usize>,
}

/// Parameters for IPO list.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct IPOListParams {
    /// Number of records
    pub limit: Option<usize>,
}

/// Parameters for block trades.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockTradeParams {
    /// Number of records
    pub limit: Option<usize>,
}

/// Parameters for institutional research.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InstitutionalResearchParams {
    /// Number of records
    pub limit: Option<usize>,
}

/// Parameters for research reports.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchReportParams {
    /// Stock code
    pub symbol: Option<String>,
    /// Number of records
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FundHoldingsParams {
    pub symbol: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewsSearchParams {
    pub keyword: String,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_search_limit")]
    pub limit: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardListParams {
    #[serde(default = "default_board_kind")]
    pub kind: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardCodeParams {
    pub code: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardKlineParams {
    pub code: String,
    #[serde(default = "default_kline_type")]
    pub kline_type: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoardResolveParams {
    pub eastmoney_code: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexKlineParams {
    pub symbol: String,
    #[serde(default = "default_kline_type")]
    pub kline_type: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FuturesQuoteParams {
    pub secids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FuturesKlineParams {
    pub secid: String,
    #[serde(default = "default_kline_type")]
    pub kline_type: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptionsListParams {
    #[serde(default = "default_option_exchange")]
    pub exchange: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptionCodesParams {
    pub codes: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FinancialParams {
    pub symbol: String,
    #[serde(default = "default_financial_kind")]
    pub kind: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PoolParams {
    #[serde(default = "default_pool_kind")]
    pub kind: String,
    pub date: Option<String>,
    pub limit: Option<usize>,
}

fn default_search_limit() -> u32 {
    10
}

fn default_board_kind() -> String {
    "industry".to_string()
}

fn default_option_exchange() -> String {
    "sse".to_string()
}

fn default_financial_kind() -> String {
    "income".to_string()
}

fn default_pool_kind() -> String {
    "limit-up".to_string()
}

fn default_forecast_limit() -> u32 {
    50
}

fn default_news_category() -> String {
    "finance".to_string()
}

fn default_page() -> u32 {
    1
}

fn default_news_limit() -> u32 {
    20
}


fn default_kline_type() -> String {
    "daily".to_string()
}

fn default_ticks_limit() -> usize {
    50
}


#[tool_router]
impl TenkMCPServer {
    #[tool(description = "Get current stock quotes for one or more symbols")]
    async fn stock_quote(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        let data = self.client
            .get_market_current(&refs)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct StockQuote {
            code: String,
            name: String,
            price: f64,
            change_pct: f64,
            volume: u64,
            amount: f64,
            high: Option<f64>,
            low: Option<f64>,
            open: Option<f64>,
            pre_close: Option<f64>,
        }

        let output: Vec<StockQuote> = data
            .iter()
            .map(|d| StockQuote {
                code: d.stock_code.clone(),
                name: d.short_name.clone(),
                price: d.price,
                change_pct: d.change_pct,
                volume: d.volume,
                amount: d.amount,
                high: d.high,
                low: d.low,
                open: d.open,
                pre_close: d.pre_close,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get historical K-line data for a stock")]
    async fn stock_kline(
        &self,
        params: Parameters<KlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let kline_type = KLineType::from_name(&params.0.kline_type);

        let mut data = self.client
            .get_market(
                &params.0.symbol,
                params.0.start.as_deref(),
                params.0.end.as_deref(),
                kline_type,
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(n) = params.0.limit {
            let len = data.len();
            if n < len {
                data = data.split_off(len - n);
            }
        }

        #[derive(Serialize)]
        struct Kline {
            date: String,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: u64,
            amount: f64,
            change_pct: f64,
        }

        let output: Vec<Kline> = data
            .iter()
            .map(|d| Kline {
                date: d.trade_date.to_string(),
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                volume: d.volume,
                amount: d.amount,
                change_pct: d.change_pct,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get intraday minute-level data for a stock")]
    async fn stock_minute(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_market_min(&params.0.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct MinuteBar {
            time: String,
            price: f64,
            avg_price: f64,
            change_pct: f64,
            volume: u64,
            amount: f64,
        }

        let output: Vec<MinuteBar> = data
            .iter()
            .map(|d| MinuteBar {
                time: d.trade_time.format("%H:%M").to_string(),
                price: d.price,
                avg_price: d.avg_price,
                change_pct: d.change_pct,
                volume: d.volume,
                amount: d.amount,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get order book for a stock")]
    async fn stock_orderbook(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_order_book(&params.0.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct OrderBook {
            code: String,
            name: String,
            bids: Vec<PriceLevel>,
            asks: Vec<PriceLevel>,
        }

        #[derive(Serialize)]
        struct PriceLevel {
            price: f64,
            volume: u64,
        }

        let output = OrderBook {
            code: data.stock_code.clone(),
            name: data.short_name.clone(),
            bids: data
                .buy_prices
                .iter()
                .zip(data.buy_volumes.iter())
                .filter(|(p, _)| **p > 0.0)
                .map(|(p, v)| PriceLevel {
                    price: *p,
                    volume: *v,
                })
                .collect(),
            asks: data
                .sell_prices
                .iter()
                .zip(data.sell_volumes.iter())
                .filter(|(p, _)| **p > 0.0)
                .map(|(p, v)| PriceLevel {
                    price: *p,
                    volume: *v,
                })
                .collect(),
        };

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get recent tick-by-tick trades for a stock")]
    async fn stock_ticks(
        &self,
        params: Parameters<TicksParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut data = self.client
            .get_ticks(&params.0.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        data.truncate(params.0.limit);

        #[derive(Serialize)]
        struct Tick {
            time: String,
            price: f64,
            volume: u64,
            direction: String,
        }

        let output: Vec<Tick> = data
            .iter()
            .map(|d| Tick {
                time: d.trade_time.format("%H:%M:%S").to_string(),
                price: d.price,
                volume: d.volume,
                direction: match d.direction {
                    'B' | 'b' => "BUY".to_string(),
                    'S' | 's' => "SELL".to_string(),
                    _ => "N/A".to_string(),
                },
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get detailed information about a stock")]
    async fn stock_info(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_stock_info(&params.0.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct StockInfo {
            code: String,
            name: String,
            full_name: String,
            exchange: String,
            industry: Option<String>,
            total_shares: Option<u64>,
            circulating_shares: Option<u64>,
            list_date: Option<String>,
        }

        let output = StockInfo {
            code: data.stock_code.clone(),
            name: data.short_name.clone(),
            full_name: data.full_name.clone(),
            exchange: data.exchange.to_string(),
            industry: data.industry.clone(),
            total_shares: data.total_shares,
            circulating_shares: data.circulating_shares,
            list_date: data.list_date.map(|d| d.to_string()),
        };

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "List all available stock codes")]
    async fn stock_list(&self, params: Parameters<ListParams>) -> Result<CallToolResult, McpError> {
        let fetch_limit = if params.0.exchange.is_some() {
            None
        } else {
            params.0.limit
        };
        let mut data = self.client
            .get_all_codes(fetch_limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(ex) = &params.0.exchange {
            let ex_upper = ex.to_uppercase();
            data.retain(|c| c.exchange.to_string() == ex_upper);
            if let Some(n) = params.0.limit {
                data.truncate(n);
            }
        }

        #[derive(Serialize)]
        struct StockCode {
            code: String,
            name: String,
            exchange: String,
        }

        let output: Vec<StockCode> = data
            .iter()
            .map(|d| StockCode {
                code: d.stock_code.clone(),
                name: d.short_name.clone(),
                exchange: d.exchange.to_string(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get current ETF quotes for one or more symbols")]
    async fn etf_quote(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        let data = self.client
            .get_etf_current(&refs)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct ETFQuote {
            code: String,
            name: String,
            price: f64,
            change_pct: Option<f64>,
            volume: u64,
            amount: f64,
        }

        let output: Vec<ETFQuote> = data
            .iter()
            .map(|d| ETFQuote {
                code: d.fund_code.clone(),
                name: d.short_name.clone(),
                price: d.price,
                change_pct: d.change_pct,
                volume: d.volume,
                amount: d.amount,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get historical K-line data for an ETF")]
    async fn etf_kline(&self, params: Parameters<KlineParams>) -> Result<CallToolResult, McpError> {
        let kline_type = KLineType::from_name(&params.0.kline_type);

        let mut data = self.client
            .get_etf_market(
                &params.0.symbol,
                params.0.start.as_deref(),
                params.0.end.as_deref(),
                kline_type,
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(n) = params.0.limit {
            let len = data.len();
            if n < len {
                data = data.split_off(len - n);
            }
        }

        #[derive(Serialize)]
        struct ETFKline {
            date: String,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: u64,
            amount: f64,
            change_pct: Option<f64>,
        }

        let output: Vec<ETFKline> = data
            .iter()
            .map(|d| ETFKline {
                date: d.trade_date.to_string(),
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                volume: d.volume,
                amount: d.amount,
                change_pct: d.change_pct,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get intraday minute-level data for an ETF")]
    async fn etf_minute(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_etf_min(&params.0.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct ETFMinuteBar {
            time: String,
            price: f64,
            avg_price: f64,
            change_pct: f64,
            volume: u64,
            amount: f64,
        }

        let output: Vec<ETFMinuteBar> = data
            .iter()
            .map(|d| ETFMinuteBar {
                time: d.trade_time.format("%H:%M").to_string(),
                price: d.price,
                avg_price: d.avg_price,
                change_pct: d.change_pct,
                volume: d.volume,
                amount: d.amount,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "List all available ETF codes")]
    async fn etf_list(&self, params: Parameters<ListParams>) -> Result<CallToolResult, McpError> {
        let fetch_limit = if params.0.exchange.is_some() {
            None
        } else {
            params.0.limit
        };
        let mut data = self.client
            .get_all_etf_codes(fetch_limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(ex) = &params.0.exchange {
            let ex_upper = ex.to_uppercase();
            data.retain(|c| c.exchange.to_string() == ex_upper);
            if let Some(n) = params.0.limit {
                data.truncate(n);
            }
        }

        #[derive(Serialize)]
        struct ETFCode {
            code: String,
            name: String,
            exchange: String,
            nav: Option<f64>,
        }

        let output: Vec<ETFCode> = data
            .iter()
            .map(|d| ETFCode {
                code: d.fund_code.clone(),
                name: d.short_name.clone(),
                exchange: d.exchange.to_string(),
                nav: d.net_value,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(
        description = "Get current bond quotes with optional filtering for top gainers/losers/volume"
    )]
    async fn bond_quote(
        &self,
        params: Parameters<BondQuoteParams>,
    ) -> Result<CallToolResult, McpError> {

        let codes: Option<Vec<&str>> = if params.0.symbols.is_empty() {
            None
        } else {
            Some(params.0.symbols.iter().map(|s| s.as_str()).collect())
        };

        let mut data = self.client
            .get_bond_current(codes.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let filter_type = if let Some(n) = params.0.top_gainers {
            data.retain(|b| b.change_pct > 0.0 && b.price > 0.0);
            data.sort_by(|a, b| {
                b.change_pct
                    .partial_cmp(&a.change_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            data.truncate(n);
            Some("top_gainers")
        } else if let Some(n) = params.0.top_losers {
            data.retain(|b| b.change_pct < 0.0 && b.price > 0.0);
            data.sort_by(|a, b| {
                a.change_pct
                    .partial_cmp(&b.change_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            data.truncate(n);
            Some("top_losers")
        } else if let Some(n) = params.0.top_volume {
            data.retain(|b| b.volume > 0 && b.price > 0.0);
            data.sort_by(|a, b| b.volume.cmp(&a.volume));
            data.truncate(n);
            Some("top_volume")
        } else {
            None
        };

        #[derive(Serialize)]
        struct Output {
            filter: Option<String>,
            bonds: Vec<BondQuote>,
        }

        #[derive(Serialize)]
        struct BondQuote {
            code: String,
            name: String,
            price: f64,
            change_pct: f64,
            volume: u64,
            amount: f64,
        }

        let output = Output {
            filter: filter_type.map(|s| s.to_string()),
            bonds: data
                .iter()
                .map(|d| BondQuote {
                    code: d.bond_code.clone(),
                    name: d.bond_name.clone(),
                    price: d.price,
                    change_pct: d.change_pct,
                    volume: d.volume,
                    amount: d.amount,
                })
                .collect(),
        };

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "List all available convertible bond codes")]
    async fn bond_list(&self, params: Parameters<LimitParam>) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_all_bond_codes(params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct BondCode {
            bond_code: String,
            bond_name: String,
            stock_code: String,
            convert_price: Option<f64>,
            list_date: Option<String>,
        }

        let output: Vec<BondCode> = data
            .iter()
            .map(|d| BondCode {
                bond_code: d.bond_code.clone(),
                bond_name: d.bond_name.clone(),
                stock_code: d.stock_code.clone(),
                convert_price: d.convert_price,
                list_date: d.listing_date.map(|d| d.to_string()),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get latest finance news by category")]
    async fn news_list(
        &self,
        params: Parameters<NewsListParams>,
    ) -> Result<CallToolResult, McpError> {
        let category = NewsCategory::from_name(&params.0.category);

        let data = self.client
            .get_news(category, params.0.page, params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct NewsArticle {
            id: String,
            title: String,
            digest: String,
            url: String,
            source: String,
            publish_time: String,
            category: String,
        }

        let output: Vec<NewsArticle> = data
            .iter()
            .map(|d| NewsArticle {
                id: d.id.clone(),
                title: d.title.clone(),
                digest: d.digest.clone(),
                url: d.url.clone(),
                source: d.source.clone(),
                publish_time: d.publish_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                category: d.category.to_string(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Read full news content by ID")]
    async fn news_read(&self, params: Parameters<NewsIdParam>) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_news_content(&params.0.id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct NewsContent {
            id: String,
            title: String,
            description: String,
            content: String,
            source: String,
            author: Option<String>,
            publish_time: String,
            related_stocks: Vec<RelatedStock>,
            related_sectors: Vec<String>,
        }

        let (stocks, sectors) = format_related_stocks(&data.related_stocks);

        let output = NewsContent {
            id: data.id,
            title: data.title,
            description: data.description,
            content: data.body_text,
            source: data.source,
            author: data.author,
            publish_time: data.publish_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            related_stocks: stocks,
            related_sectors: sectors,
        };

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get real-time capital flow data for stocks (main vs retail money flow)")]
    async fn capital_flow(
        &self,
        params: Parameters<CapitalFlowParams>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();

        let data = self.client
            .get_capital_flow(&refs)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Flow {
            code: String,
            name: String,
            main_net_inflow: f64,
            main_inflow: f64,
            main_outflow: f64,
            super_large_net: f64,
            large_net: f64,
            medium_net: f64,
            small_net: f64,
            main_net_ratio: f64,
        }

        let output: Vec<Flow> = data
            .iter()
            .map(|d| Flow {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                main_net_inflow: d.main_net_inflow,
                main_inflow: d.main_inflow,
                main_outflow: d.main_outflow,
                super_large_net: d.super_large_net_inflow,
                large_net: d.large_net_inflow,
                medium_net: d.medium_net_inflow,
                small_net: d.small_net_inflow,
                main_net_ratio: d.main_net_ratio,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get historical capital flow data for a stock")]
    async fn capital_flow_history(
        &self,
        params: Parameters<CapitalFlowHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_capital_flow_history(&params.0.symbol, params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct FlowHistory {
            date: String,
            main_net: f64,
            small_net: f64,
            medium_net: f64,
            large_net: f64,
            super_large_net: f64,
            close: f64,
            change_pct: f64,
        }

        let output: Vec<FlowHistory> = data
            .iter()
            .map(|d| FlowHistory {
                date: d.trade_date.to_string(),
                main_net: d.main_net_inflow,
                small_net: d.small_net_inflow,
                medium_net: d.medium_net_inflow,
                large_net: d.large_net_inflow,
                super_large_net: d.super_large_net_inflow,
                close: d.close,
                change_pct: d.change_pct,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get Billboard list")]
    async fn billboard_list(
        &self,
        params: Parameters<BillboardParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_billboard_list(params.0.date.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Billboard {
            code: String,
            name: String,
            date: String,
            close: f64,
            change_pct: f64,
            turnover_ratio: f64,
            net_buy_amount: f64,
            buy_amount: f64,
            sell_amount: f64,
            reason: String,
        }

        let output: Vec<Billboard> = data
            .iter()
            .map(|d| Billboard {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                date: d.trade_date.to_string(),
                close: d.close,
                change_pct: d.change_pct,
                turnover_ratio: d.turnover_ratio,
                net_buy_amount: d.net_buy_amount,
                buy_amount: d.buy_amount,
                sell_amount: d.sell_amount,
                reason: d.reason.clone(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get Billboard details for a stock")]
    async fn billboard_detail(
        &self,
        params: Parameters<BillboardDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_billboard_detail(&params.0.symbol, &params.0.date)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Detail {
            code: String,
            date: String,
            trader: String,
            buy_amount: f64,
            sell_amount: f64,
            net_amount: f64,
            direction: String,
        }

        let output: Vec<Detail> = data
            .iter()
            .map(|d| Detail {
                code: d.stock_code.clone(),
                date: d.trade_date.to_string(),
                trader: d.trader_name.clone(),
                buy_amount: d.buy_amount,
                sell_amount: d.sell_amount,
                net_amount: d.net_amount,
                direction: d.direction.clone(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get earnings forecast data")]
    async fn earnings_forecast(
        &self,
        params: Parameters<EarningsForecastParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_earnings_forecast(
                params.0.report_period.as_deref(),
                params.0.page,
                params.0.limit,
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Forecast {
            code: String,
            name: String,
            forecast_type: String,
            profit_min: Option<f64>,
            profit_max: Option<f64>,
            change_min: Option<f64>,
            change_max: Option<f64>,
            report_period: String,
            announce_date: String,
            summary: Option<String>,
        }

        let output: Vec<Forecast> = data
            .iter()
            .map(|d| Forecast {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                forecast_type: d.forecast_type.clone(),
                profit_min: d.profit_min,
                profit_max: d.profit_max,
                change_min: d.change_min,
                change_max: d.change_max,
                report_period: d.report_period.clone(),
                announce_date: d.announce_date.to_string(),
                summary: d.summary.clone(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get Stock Connect data")]
    async fn stock_connect(
        &self,
        params: Parameters<StockConnectParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_stock_connect(params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Connect {
            date: String,
            north_net_buy: f64,
            sh_net_buy: f64,
            sz_net_buy: f64,
            north_buy: f64,
            north_sell: f64,
        }

        let output: Vec<Connect> = data
            .iter()
            .map(|d| Connect {
                date: d.trade_date.to_string(),
                north_net_buy: d.north_net_buy,
                sh_net_buy: d.sh_net_buy,
                sz_net_buy: d.sz_net_buy,
                north_buy: d.north_buy,
                north_sell: d.north_sell,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get margin trading data for a stock")]
    async fn margin_trading(
        &self,
        params: Parameters<MarginTradingParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_margin_trading(&params.0.symbol, params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Margin {
            code: String,
            name: String,
            date: String,
            margin_balance: f64,
            margin_buy: f64,
            margin_repay: f64,
            short_balance: f64,
            short_volume: u64,
            total_balance: f64,
        }

        let output: Vec<Margin> = data
            .iter()
            .map(|d| Margin {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                date: d.trade_date.to_string(),
                margin_balance: d.margin_balance,
                margin_buy: d.margin_buy,
                margin_repay: d.margin_repay,
                short_balance: d.short_balance,
                short_volume: d.short_volume,
                total_balance: d.total_balance,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get IPO list")]
    async fn ipo_list(
        &self,
        params: Parameters<IPOListParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_ipo_list(params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct IPO {
            code: String,
            name: String,
            issue_price: f64,
            sub_date: String,
            list_date: Option<String>,
            winning_rate: Option<f64>,
            issue_quantity: Option<u64>,
            pe_ratio: Option<f64>,
        }

        let output: Vec<IPO> = data
            .iter()
            .map(|d| IPO {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                issue_price: d.issue_price,
                sub_date: d.sub_date.to_string(),
                list_date: d.list_date.map(|d| d.to_string()),
                winning_rate: d.winning_rate,
                issue_quantity: d.issue_quantity,
                pe_ratio: d.pe_ratio,
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get block trade list")]
    async fn block_trades(
        &self,
        params: Parameters<BlockTradeParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_block_trades(params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct BlockTrade {
            code: String,
            name: String,
            date: String,
            price: f64,
            close_price: f64,
            premium_rate: f64,
            volume: u64,
            amount: f64,
            buyer: String,
            seller: String,
        }

        let output: Vec<BlockTrade> = data
            .iter()
            .map(|d| BlockTrade {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                date: d.trade_date.to_string(),
                price: d.price,
                close_price: d.close_price,
                premium_rate: d.premium_rate,
                volume: d.volume,
                amount: d.amount,
                buyer: d.buyer.clone(),
                seller: d.seller.clone(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get institutional research list")]
    async fn institutional_research(
        &self,
        params: Parameters<InstitutionalResearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_institutional_research(params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Research {
            code: String,
            name: String,
            date: String,
            institution_count: u32,
            institutions: String,
            research_type: String,
            researchers: Option<String>,
        }

        let output: Vec<Research> = data
            .iter()
            .map(|d| Research {
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                date: d.research_date.to_string(),
                institution_count: d.institution_count,
                institutions: d.institutions.clone(),
                research_type: d.research_type.clone(),
                researchers: d.researchers.clone(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Get research reports")]
    async fn research_reports(
        &self,
        params: Parameters<ResearchReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let data = self.client
            .get_research_reports(params.0.symbol.as_deref(), params.0.limit)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        #[derive(Serialize)]
        struct Report {
            id: String,
            code: String,
            name: String,
            title: String,
            institution: String,
            analysts: String,
            rating: Option<String>,
            date: String,
        }

        let output: Vec<Report> = data
            .iter()
            .map(|d| Report {
                id: d.report_id.clone(),
                code: d.stock_code.clone(),
                name: d.stock_name.clone(),
                title: d.title.clone(),
                institution: d.institution.clone(),
                analysts: d.analysts.clone(),
                rating: d.rating.clone(),
                date: d.publish_date.to_string(),
            })
            .collect();

        Self::ok(Self::to_json(&output)?)
    }

    #[tool(description = "Search finance news by keyword")]
    async fn news_search(
        &self,
        params: Parameters<NewsSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let keyword = params.0.keyword.clone();
        let page = params.0.page;
        let limit = params.0.limit;
        Self::json_result(self.client.search_news(&keyword, page, limit)).await
    }

    #[tool(description = "Get stock valuation metrics")]
    async fn stock_valuation(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let symbol = params.0.symbol.clone();
        Self::json_result(self.client.get_valuation(&symbol)).await
    }

    #[tool(description = "Get top 10 shareholders")]
    async fn stock_holders(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let symbol = params.0.symbol.clone();
        Self::json_result(self.client.get_top_holders(&symbol)).await
    }

    #[tool(description = "Get fund holdings for a stock")]
    async fn stock_funds(
        &self,
        params: Parameters<FundHoldingsParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbol = params.0.symbol.clone();
        let limit = params.0.limit;
        Self::json_result(self.client.get_fund_holdings(&symbol, limit)).await
    }

    #[tool(description = "Get dividend history")]
    async fn stock_dividends(
        &self,
        params: Parameters<SymbolParam>,
    ) -> Result<CallToolResult, McpError> {
        let symbol = params.0.symbol.clone();
        Self::json_result(self.client.get_dividends(&symbol)).await
    }

    #[tool(description = "List index codes")]
    async fn index_list(
        &self,
        params: Parameters<LimitParam>,
    ) -> Result<CallToolResult, McpError> {
        Self::json_result(self.client.get_index_list(params.0.limit)).await
    }

    #[tool(description = "Get index quotes")]
    async fn index_quote(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        Self::json_result(self.client.get_index_current(&refs)).await
    }

    #[tool(description = "Get index K-line data")]
    async fn index_kline(
        &self,
        params: Parameters<IndexKlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let kline_type = KLineType::from_name(&params.0.kline_type);
        let symbol = params.0.symbol.clone();
        let start = params.0.start.clone();
        let end = params.0.end.clone();
        let limit = params.0.limit;
        Self::json_result(async {
            Ok(limit_kline(
                self.client
                    .get_index_market(&symbol, start.as_deref(), end.as_deref(), kline_type)
                    .await?,
                limit,
            ))
        })
        .await
    }

    #[tool(description = "List industry or concept boards")]
    async fn board_list(
        &self,
        params: Parameters<BoardListParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = BoardKindArg::parse(&params.0.kind);
        let limit = params.0.limit;
        Self::json_result(async {
            match kind {
                BoardKindArg::Industry => self.client.get_industry_boards(limit).await,
                BoardKindArg::Concept => self.client.get_concept_boards(limit).await,
            }
        })
        .await
    }

    #[tool(description = "Get board K-line data")]
    async fn board_kline(
        &self,
        params: Parameters<BoardKlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let kline_type = KLineType::from_name(&params.0.kline_type);
        let code = params.0.code.clone();
        let start = params.0.start.clone();
        let end = params.0.end.clone();
        let limit = params.0.limit;
        Self::json_result(async {
            Ok(limit_kline(
                self.client
                    .get_board_market(&code, start.as_deref(), end.as_deref(), kline_type)
                    .await?,
                limit,
            ))
        })
        .await
    }

    #[tool(description = "Get board constituent stocks")]
    async fn board_members(
        &self,
        params: Parameters<BoardCodeParams>,
    ) -> Result<CallToolResult, McpError> {
        let code = params.0.code.clone();
        let limit = params.0.limit;
        Self::json_result(self.client.get_board_constituents(&code, limit))
            .await
    }

    #[tool(description = "Map EastMoney and THS board codes by name")]
    async fn board_crosswalk(
        &self,
        params: Parameters<BoardListParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = BoardKindArg::parse(&params.0.kind);
        let limit = params.0.limit;
        Self::json_result(self.client.resolve_board_crosswalk(kind.into(), limit))
            .await
    }

    #[tool(description = "Resolve THS board code for an EastMoney board via constituent overlap")]
    async fn board_resolve(
        &self,
        params: Parameters<BoardResolveParams>,
    ) -> Result<CallToolResult, McpError> {
        let code = params.0.eastmoney_code.clone();
        let limit = params.0.limit;
        Self::json_result(self.client.resolve_ths_board_for_eastmoney(&code, limit))
            .await
    }

    #[tool(description = "List futures contracts")]
    async fn futures_list(
        &self,
        params: Parameters<LimitParam>,
    ) -> Result<CallToolResult, McpError> {
        Self::json_result(self.client.get_futures_list(params.0.limit)).await
    }

    #[tool(description = "Get futures quotes by secid or symbol")]
    async fn futures_quote(
        &self,
        params: Parameters<FuturesQuoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.secids.iter().map(|s| s.as_str()).collect();
        Self::json_result(self.client.get_futures_current(&refs)).await
    }

    #[tool(description = "Get futures K-line data")]
    async fn futures_kline(
        &self,
        params: Parameters<FuturesKlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let kline_type = KLineType::from_name(&params.0.kline_type);
        let secid = params.0.secid.clone();
        let start = params.0.start.clone();
        let end = params.0.end.clone();
        let limit = params.0.limit;
        Self::json_result(async {
            Ok(limit_kline(
                self.client
                    .get_futures_market(&secid, start.as_deref(), end.as_deref(), kline_type)
                    .await?,
                limit,
            ))
        })
        .await
    }

    #[tool(description = "List exchange-traded options")]
    async fn options_list(
        &self,
        params: Parameters<OptionsListParams>,
    ) -> Result<CallToolResult, McpError> {
        let exchange = OptionExchangeArg::parse(&params.0.exchange);
        let limit = params.0.limit;
        Self::json_result(self.client.get_options_list(exchange.into(), limit))
            .await
    }

    #[tool(description = "Get option quotes")]
    async fn options_quote(
        &self,
        params: Parameters<OptionCodesParams>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.codes.iter().map(|s| s.as_str()).collect();
        Self::json_result(self.client.get_options_current(&refs)).await
    }

    #[tool(description = "Get financial statements (balance, income, cashflow, performance)")]
    async fn financial_statement(
        &self,
        params: Parameters<FinancialParams>,
    ) -> Result<CallToolResult, McpError> {
        let symbol = params.0.symbol.clone();
        let kind = FinancialKindArg::parse(&params.0.kind);
        let limit = params.0.limit;
        Self::json_result(self.client.get_financial_statement(&symbol, kind.into(), limit))
            .await
    }

    #[tool(description = "Get CPI macro data")]
    async fn macro_cpi(
        &self,
        params: Parameters<LimitParam>,
    ) -> Result<CallToolResult, McpError> {
        Self::json_result(self.client.get_macro_cpi(params.0.limit)).await
    }

    #[tool(description = "Get GDP macro data")]
    async fn macro_gdp(
        &self,
        params: Parameters<LimitParam>,
    ) -> Result<CallToolResult, McpError> {
        Self::json_result(self.client.get_macro_gdp(params.0.limit)).await
    }

    #[tool(description = "Get Hong Kong stock quotes")]
    async fn global_hk(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        Self::json_result(self.client.get_hk_current(&refs)).await
    }

    #[tool(description = "Get US stock quotes")]
    async fn global_us(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        Self::json_result(self.client.get_us_current(&refs)).await
    }

    #[tool(description = "Get limit-up/limit-down pool stocks")]
    async fn limit_pool(
        &self,
        params: Parameters<PoolParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = PoolKindArg::parse(&params.0.kind);
        let date = params.0.date.clone();
        let limit = params.0.limit;
        Self::json_result(self.client.get_limit_pool(kind.into(), date.as_deref(), limit))
            .await
    }
}

#[tool_handler(
    name = "tenk-mcp",
    instructions = "Chinese market data: stocks, ETFs, bonds, indices, boards, futures, options, financials, news, and market analytics"
)]
impl ServerHandler for TenkMCPServer {}

/// Runs the MCP server.
pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::transport::io::stdio;

    tracing::info!("Starting tenk MCP server");

    let server = TenkMCPServer::new();
    let service = rmcp::serve_server(server, stdio()).await?;

    tracing::info!("tenk MCP server ready");
    service.waiting().await?;

    Ok(())
}
