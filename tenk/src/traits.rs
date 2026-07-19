//! Traits for data sources.

use async_trait::async_trait;

use crate::data::{
    BillboardDetail, BillboardItem, BlockTradeData, BondCurrentData, BoardItem, CapitalFlowData,
    CapitalFlowHistory, ConvertibleBondCode, CurrentMarketData, DerivativesQuote,
    DividendData, ETFCode, ETFCurrentData, ETFMarketData, ETFMinuteData, EarningsForecast,
    FinancialRecord, FinancialReportKind, FundHolding, FuturesContract, IndexCode, IPOData,
    InstitutionalResearchData, KLineType, LimitPoolItem, LimitPoolKind, MacroRecord,
    MarginTradingData, MarketData, MinuteData, NewsArticle, NewsCategory, NewsContent,
    OptionContract, OptionExchange, OrderBookData, ResearchReportData, StockCode,
    StockConnectData, StockInfo, StockValuation, TickData, TopHolder,
};
use crate::error::DataResult;

/// Base trait for data sources.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Returns the source name.
    fn name(&self) -> &'static str;
    /// Returns the source priority (lower is higher priority).
    fn priority(&self) -> u8;
    /// Checks if the source is available.
    async fn is_available(&self) -> bool {
        true
    }
}

/// Stock market data source.
#[async_trait]
pub trait StockMarketSource: DataSource {
    /// Fetches historical K-line market data.
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>>;

    /// Fetches real-time market quotes.
    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>>;

    /// Fetches intraday minute-level data.
    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>>;

    /// Fetches order book data.
    async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        let _ = stock_code;
        Err(crate::error::DataError::not_supported("get_order_book"))
    }

    /// Fetches tick-by-tick trade data.
    async fn get_ticks(&self, stock_code: &str) -> DataResult<Vec<TickData>> {
        let _ = stock_code;
        Err(crate::error::DataError::not_supported("get_ticks"))
    }
}

/// Stock info source.
#[async_trait]
pub trait StockInfoSource: DataSource {
    /// Gets all stock codes, optionally limited.
    async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>>;

    /// Fetches detailed stock information.
    async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        let _ = stock_code;
        Err(crate::error::DataError::not_supported("get_stock_info"))
    }
}

/// Combined stock market and info source.
pub trait FullStockSource: StockMarketSource + StockInfoSource {}
impl<T: StockMarketSource + StockInfoSource> FullStockSource for T {}

/// ETF info source.
#[async_trait]
pub trait FundInfoSource: DataSource {
    /// Gets all ETF codes, optionally limited.
    async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>>;
}

/// ETF market data source.
#[async_trait]
pub trait FundMarketSource: DataSource {
    /// Fetches historical ETF K-line market data.
    async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>>;

    /// Fetches real-time ETF quotes.
    async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>>;

    /// Fetches intraday ETF minute-level data.
    async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        let _ = fund_code;
        Err(crate::error::DataError::not_supported("get_etf_min"))
    }
}

/// Combined ETF info and market source.
pub trait FullFundSource: FundInfoSource + FundMarketSource {}
impl<T: FundInfoSource + FundMarketSource> FullFundSource for T {}

/// Bond info source.
#[async_trait]
pub trait BondInfoSource: DataSource {
    /// Gets all bond codes, optionally limited.
    async fn get_all_bond_codes(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<ConvertibleBondCode>>;
}

/// Bond market data source.
#[async_trait]
pub trait BondMarketSource: DataSource {
    /// Fetches real-time bond quotes.
    async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>>;
}

/// Combined bond info and market source.
pub trait FullBondSource: BondInfoSource + BondMarketSource {}
impl<T: BondInfoSource + BondMarketSource> FullBondSource for T {}

/// News source.
#[async_trait]
pub trait NewsSource: DataSource {
    /// Get latest news by category.
    async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>>;

    /// Get full news content by ID.
    async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent>;

    /// Search news by keyword.
    async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        let _ = (keyword, page, limit);
        Err(crate::error::DataError::not_supported("search_news"))
    }
}

/// Capital flow data source.
#[async_trait]
pub trait CapitalFlowSource: DataSource {
    /// Get real-time capital flow for stocks.
    async fn get_capital_flow(&self, stock_codes: &[&str]) -> DataResult<Vec<CapitalFlowData>>;

    /// Get historical capital flow for a stock.
    async fn get_capital_flow_history(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<CapitalFlowHistory>>;
}

/// Billboard data source.
#[async_trait]
pub trait BillboardSource: DataSource {
    /// Get Billboard list by date.
    async fn get_billboard_list(&self, date: Option<&str>) -> DataResult<Vec<BillboardItem>>;

    /// Get Billboard details for a stock.
    async fn get_billboard_detail(
        &self,
        stock_code: &str,
        date: &str,
    ) -> DataResult<Vec<BillboardDetail>>;
}

/// Earnings forecast data source.
#[async_trait]
pub trait EarningsForecastSource: DataSource {
    /// Get earnings forecasts.
    async fn get_earnings_forecast(
        &self,
        report_period: Option<&str>,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<EarningsForecast>>;
}

/// Stock Connect data source.
#[async_trait]
pub trait StockConnectSource: DataSource {
    /// Get Stock Connect daily flow data.
    async fn get_stock_connect(&self, limit: Option<usize>) -> DataResult<Vec<StockConnectData>>;
}

/// Margin trading data source.
#[async_trait]
pub trait MarginTradingSource: DataSource {
    /// Get margin trading data for a stock.
    async fn get_margin_trading(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<MarginTradingData>>;
}

/// IPO data source.
#[async_trait]
pub trait IPOSource: DataSource {
    /// Get IPO list.
    async fn get_ipo_list(&self, limit: Option<usize>) -> DataResult<Vec<IPOData>>;
}

/// Block trade data source.
#[async_trait]
pub trait BlockTradeSource: DataSource {
    /// Get block trade list.
    async fn get_block_trades(&self, limit: Option<usize>) -> DataResult<Vec<BlockTradeData>>;
}

/// Institutional research data source.
#[async_trait]
pub trait InstitutionalResearchSource: DataSource {
    /// Get institutional research list.
    async fn get_institutional_research(
        &self,
        limit: Option<usize>,
    ) -> DataResult<Vec<InstitutionalResearchData>>;
}

/// Research report data source.
#[async_trait]
pub trait ResearchReportSource: DataSource {
    /// Get research reports.
    async fn get_research_reports(
        &self,
        stock_code: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<ResearchReportData>>;
}

/// Stock valuation data source.
#[async_trait]
pub trait ValuationSource: DataSource {
    /// Get stock valuation metrics.
    async fn get_valuation(&self, stock_code: &str) -> DataResult<StockValuation>;
}

/// Stock holdings data source.
#[async_trait]
pub trait HoldingsSource: DataSource {
    /// Get top 10 shareholders.
    async fn get_top_holders(&self, stock_code: &str) -> DataResult<Vec<TopHolder>>;

    /// Get fund holdings for a stock.
    async fn get_fund_holdings(
        &self,
        stock_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<FundHolding>>;
}

/// Dividend data source.
#[async_trait]
pub trait DividendSource: DataSource {
    /// Get dividend history.
    async fn get_dividends(&self, stock_code: &str) -> DataResult<Vec<DividendData>>;
}

/// Index market data source.
#[async_trait]
pub trait IndexMarketSource: DataSource {
    async fn get_index_list(&self, limit: Option<usize>) -> DataResult<Vec<IndexCode>>;

    async fn get_index_current(&self, index_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let _ = index_codes;
        Err(crate::error::DataError::not_supported("get_index_current"))
    }

    async fn get_index_market(
        &self,
        index_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let _ = (index_code, start_date, end_date, k_type);
        Err(crate::error::DataError::not_supported("get_index_market"))
    }
}

/// Sector / concept board source.
#[async_trait]
pub trait BoardMarketSource: DataSource {
    async fn get_industry_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        let _ = limit;
        Err(crate::error::DataError::not_supported("get_industry_boards"))
    }

    async fn get_concept_boards(&self, limit: Option<usize>) -> DataResult<Vec<BoardItem>> {
        let _ = limit;
        Err(crate::error::DataError::not_supported("get_concept_boards"))
    }

    async fn get_board_market(
        &self,
        board_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let _ = (board_code, start_date, end_date, k_type);
        Err(crate::error::DataError::not_supported("get_board_market"))
    }

    async fn get_board_constituents(
        &self,
        board_code: &str,
        limit: Option<usize>,
    ) -> DataResult<Vec<StockCode>> {
        let _ = (board_code, limit);
        Err(crate::error::DataError::not_supported("get_board_constituents"))
    }
}

/// Limit-up / limit-down pool source.
#[async_trait]
pub trait LimitPoolSource: DataSource {
    async fn get_limit_pool(
        &self,
        kind: LimitPoolKind,
        date: Option<&str>,
        limit: Option<usize>,
    ) -> DataResult<Vec<LimitPoolItem>> {
        let _ = (kind, date, limit);
        Err(crate::error::DataError::not_supported("get_limit_pool"))
    }
}

/// Macro economic data source.
#[async_trait]
pub trait MacroSource: DataSource {
    async fn get_macro_cpi(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        let _ = limit;
        Err(crate::error::DataError::not_supported("get_macro_cpi"))
    }

    async fn get_macro_gdp(&self, limit: Option<usize>) -> DataResult<Vec<MacroRecord>> {
        let _ = limit;
        Err(crate::error::DataError::not_supported("get_macro_gdp"))
    }
}

/// Hong Kong / US quote source.
#[async_trait]
pub trait GlobalMarketSource: DataSource {
    async fn get_hk_current(&self, codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let _ = codes;
        Err(crate::error::DataError::not_supported("get_hk_current"))
    }

    async fn get_us_current(&self, symbols: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let _ = symbols;
        Err(crate::error::DataError::not_supported("get_us_current"))
    }
}

#[async_trait]
pub trait FuturesSource: DataSource {
    async fn get_futures_list(&self, limit: Option<usize>) -> DataResult<Vec<FuturesContract>> {
        let _ = limit;
        Err(crate::error::DataError::not_supported("get_futures_list"))
    }

    async fn get_futures_current(&self, secids: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        let _ = secids;
        Err(crate::error::DataError::not_supported("get_futures_current"))
    }

    async fn get_futures_market(
        &self,
        secid: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let _ = (secid, start_date, end_date, k_type);
        Err(crate::error::DataError::not_supported("get_futures_market"))
    }
}

#[async_trait]
pub trait OptionsSource: DataSource {
    async fn get_options_list(
        &self,
        exchange: OptionExchange,
        limit: Option<usize>,
    ) -> DataResult<Vec<OptionContract>> {
        let _ = (exchange, limit);
        Err(crate::error::DataError::not_supported("get_options_list"))
    }

    async fn get_options_current(&self, contract_codes: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        let _ = contract_codes;
        Err(crate::error::DataError::not_supported("get_options_current"))
    }
}

#[async_trait]
pub trait FinancialSource: DataSource {
    async fn get_financial_statement(
        &self,
        stock_code: &str,
        kind: FinancialReportKind,
        limit: Option<usize>,
    ) -> DataResult<Vec<FinancialRecord>> {
        let _ = (stock_code, kind, limit);
        Err(crate::error::DataError::not_supported("get_financial_statement"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSource;

    #[async_trait]
    impl DataSource for MockSource {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn priority(&self) -> u8 {
            1
        }
    }

    #[tokio::test]
    async fn test_mock_source() {
        let source = MockSource;
        assert_eq!(source.name(), "mock");
        assert_eq!(source.priority(), 1);
        assert!(source.is_available().await);
    }
}
