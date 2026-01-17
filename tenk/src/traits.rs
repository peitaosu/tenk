//! Traits for data sources.

use async_trait::async_trait;

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, KLineType, MarketData, MinuteData, NewsArticle, NewsCategory,
    NewsContent, OrderBookData, StockCode, StockInfo, TickData,
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

/// Index market data source.
#[async_trait]
pub trait IndexMarketSource: DataSource {
    /// Fetches historical index K-line market data.
    async fn get_index_market(
        &self,
        index_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>>;

    /// Fetches real-time index quotes.
    async fn get_index_current(&self, index_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>>;
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
    async fn get_all_bond_codes(&self, limit: Option<usize>) -> DataResult<Vec<ConvertibleBondCode>>;
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
