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

    /// K-line type (daily, weekly, etc.)
    #[serde(default = "default_kline_type")]
    pub kline_type: String,

    /// Start date (YYYY-MM-DD)
    pub start: Option<String>,

    /// End date (YYYY-MM-DD)
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

    #[tool(description = "Get order book (bid/ask levels) for a stock")]
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
        let mut data = client
            .get_all_codes()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(ex) = &params.0.exchange {
            let ex_upper = ex.to_uppercase();
            data.retain(|c| c.exchange.to_string() == ex_upper);
        }

        if let Some(n) = params.0.limit {
            data.truncate(n);
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
        let mut data = client
            .get_all_etf_codes()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(ex) = &params.0.exchange {
            let ex_upper = ex.to_uppercase();
            data.retain(|c| c.exchange.to_string() == ex_upper);
        }

        if let Some(n) = params.0.limit {
            data.truncate(n);
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
        let mut data = client
            .get_all_bond_codes()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(n) = params.0.limit {
            data.truncate(n);
        }

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

pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::transport::io::stdio;

    tracing::info!("Starting tenk MCP server");

    let server = TenkMCPServer::new();
    let service = rmcp::serve_server(server, stdio()).await?;

    tracing::info!("tenk MCP server ready");
    service.waiting().await?;

    Ok(())
}
