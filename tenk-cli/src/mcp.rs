//! MCP Server for tenk.

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tenk::sources::{EastMoneySource, SinaSource, THSSource};
use tenk::traits::{
    BillboardSource, BlockTradeSource, CapitalFlowSource, EarningsForecastSource, IPOSource,
    InstitutionalResearchSource, MarginTradingSource, ResearchReportSource, StockConnectSource,
};
use tenk::{DataClient, KLineType, NewsCategory};

/// MCP server implementation.
#[derive(Clone)]
pub struct TenkMCPServer {
    /// Tool router for handling MCP requests
    tool_router: ToolRouter<Self>,
}

impl TenkMCPServer {
    /// Creates a new MCP server instance.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn build_client() -> DataClient {
        DataClient::new()
            .with_source(EastMoneySource::default())
            .with_fund_source(EastMoneySource::default())
            .with_bond_source(EastMoneySource::default())
            .with_news_source(EastMoneySource::default())
            .with_source(SinaSource::default())
            .with_fund_source(SinaSource::default())
            .with_bond_market_source(SinaSource::default())
            .with_source(THSSource::default())
            .with_fund_source(THSSource::default())
            .with_bond_info_source(THSSource::default())
    }

    fn to_json<T: Serialize>(value: &T) -> Result<String, McpError> {
        serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn ok(text: String) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
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

fn parse_news_category(s: &str) -> NewsCategory {
    match s.to_lowercase().as_str() {
        "finance" | "102" => NewsCategory::Finance,
        "company" | "103" => NewsCategory::Company,
        "stock" | "104" => NewsCategory::Stock,
        "us" | "usmarket" | "105" => NewsCategory::USMarket,
        "global" | "111" => NewsCategory::Global,
        "domestic" | "106" => NewsCategory::Domestic,
        "industry" | "115" => NewsCategory::Industry,
        _ => NewsCategory::Finance,
    }
}

fn default_kline_type() -> String {
    "daily".to_string()
}

fn default_ticks_limit() -> usize {
    50
}

fn parse_kline_type(s: &str) -> KLineType {
    match s.to_lowercase().as_str() {
        "weekly" => KLineType::Weekly,
        "monthly" => KLineType::Monthly,
        "quarterly" => KLineType::Quarterly,
        "min5" => KLineType::Min5,
        "min15" => KLineType::Min15,
        "min30" => KLineType::Min30,
        "min60" => KLineType::Min60,
        _ => KLineType::Daily,
    }
}

#[tool_router]
impl TenkMCPServer {
    #[tool(description = "Get current stock quotes for one or more symbols")]
    async fn stock_quote(
        &self,
        params: Parameters<SymbolsParam>,
    ) -> Result<CallToolResult, McpError> {
        let client = Self::build_client();
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        let data = client
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
        let client = Self::build_client();
        let kline_type = parse_kline_type(&params.0.kline_type);

        let mut data = client
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
        let client = Self::build_client();
        let data = client
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
        let client = Self::build_client();
        let data = client
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
        let client = Self::build_client();
        let mut data = client
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
        let client = Self::build_client();
        let data = client
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
        let client = Self::build_client();
        let fetch_limit = if params.0.exchange.is_some() {
            None
        } else {
            params.0.limit
        };
        let mut data = client
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
        let client = Self::build_client();
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();
        let data = client
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
        let client = Self::build_client();
        let kline_type = parse_kline_type(&params.0.kline_type);

        let mut data = client
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
        let client = Self::build_client();
        let data = client
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
        let client = Self::build_client();
        let fetch_limit = if params.0.exchange.is_some() {
            None
        } else {
            params.0.limit
        };
        let mut data = client
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
        let client = Self::build_client();

        let codes: Option<Vec<&str>> = if params.0.symbols.is_empty() {
            None
        } else {
            Some(params.0.symbols.iter().map(|s| s.as_str()).collect())
        };

        let mut data = client
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
        let client = Self::build_client();
        let data = client
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
        let client = Self::build_client();
        let category = parse_news_category(&params.0.category);

        let data = client
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
        let client = Self::build_client();

        let data = client
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
            related_stocks: Vec<RelatedStockOutput>,
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
        let source = EastMoneySource::default();
        let refs: Vec<&str> = params.0.symbols.iter().map(|s| s.as_str()).collect();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
        let source = EastMoneySource::default();

        let data = source
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
}

/// Related stock information output.
#[derive(Serialize)]
struct RelatedStockOutput {
    /// Stock symbol
    symbol: String,
    /// Market code (SH/SZ)
    market: String,
    /// Formatted symbol with market
    formatted: String,
}

/// Format related stock codes into structured format
fn format_related_stocks(codes: &[String]) -> (Vec<RelatedStockOutput>, Vec<String>) {
    let mut stocks = Vec::new();
    let mut sectors = Vec::new();

    for code in codes {
        if let Some((market, symbol)) = code.split_once('.') {
            match market {
                "0" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "SZ".to_string(),
                    formatted: format!("{}.SZ", symbol),
                }),
                "1" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "SH".to_string(),
                    formatted: format!("{}.SH", symbol),
                }),
                "90" => {
                    sectors.push(symbol.to_string());
                }
                "105" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "NASDAQ".to_string(),
                    formatted: format!("{} (NASDAQ)", symbol),
                }),
                "106" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "NYSE".to_string(),
                    formatted: format!("{} (NYSE)", symbol),
                }),
                "116" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "HK".to_string(),
                    formatted: format!("{}.HK", symbol),
                }),
                "118" => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: "KR".to_string(),
                    formatted: format!("{} (KR)", symbol),
                }),
                _ => stocks.push(RelatedStockOutput {
                    symbol: symbol.to_string(),
                    market: market.to_string(),
                    formatted: code.clone(),
                }),
            }
        }
    }

    (stocks, sectors)
}

#[tool_handler]
impl ServerHandler for TenkMCPServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "tenk-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }
}

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
