//! Data client with multi-source fallback.

use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, KLineType, MarketData, MinuteData, NewsArticle, NewsCategory,
    NewsContent, OrderBookData, StockCode, StockInfo, TickData,
};
use crate::error::{DataError, DataResult};
use crate::traits::{
    BondInfoSource, BondMarketSource, FundInfoSource, FundMarketSource, NewsSource,
    StockInfoSource, StockMarketSource,
};

/// Client for fetching financial data from multiple sources.
pub struct DataClient {
    /// Stock market data sources
    market_sources: Vec<Arc<dyn StockMarketSource>>,
    /// Stock info sources
    info_sources: Vec<Arc<dyn StockInfoSource>>,
    /// Fund/ETF info sources
    fund_info_sources: Vec<Arc<dyn FundInfoSource>>,
    /// Fund/ETF market data sources
    fund_market_sources: Vec<Arc<dyn FundMarketSource>>,
    /// Bond info sources
    bond_info_sources: Vec<Arc<dyn BondInfoSource>>,
    /// Bond market data sources
    bond_market_sources: Vec<Arc<dyn BondMarketSource>>,
    /// News sources
    news_sources: Vec<Arc<dyn NewsSource>>,
}

impl DataClient {
    /// Creates a new empty DataClient.
    pub fn new() -> Self {
        Self {
            market_sources: Vec::new(),
            info_sources: Vec::new(),
            fund_info_sources: Vec::new(),
            fund_market_sources: Vec::new(),
            bond_info_sources: Vec::new(),
            bond_market_sources: Vec::new(),
            news_sources: Vec::new(),
        }
    }

    /// Adds a stock market data source.
    pub fn with_market_source<S: StockMarketSource + 'static>(mut self, source: S) -> Self {
        self.market_sources.push(Arc::new(source));
        self.market_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a stock info source.
    pub fn with_info_source<S: StockInfoSource + 'static>(mut self, source: S) -> Self {
        self.info_sources.push(Arc::new(source));
        self.info_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a combined stock market and info source.
    pub fn with_source<S: StockMarketSource + StockInfoSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self.market_sources.push(Arc::new(source.clone()));
        self.info_sources.push(Arc::new(source));
        self.market_sources.sort_by_key(|s| s.priority());
        self.info_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds an ETF info source.
    pub fn with_fund_info_source<S: FundInfoSource + 'static>(mut self, source: S) -> Self {
        self.fund_info_sources.push(Arc::new(source));
        self.fund_info_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds an ETF market data source.
    pub fn with_fund_market_source<S: FundMarketSource + 'static>(mut self, source: S) -> Self {
        self.fund_market_sources.push(Arc::new(source));
        self.fund_market_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a bond info source.
    pub fn with_bond_info_source<S: BondInfoSource + 'static>(mut self, source: S) -> Self {
        self.bond_info_sources.push(Arc::new(source));
        self.bond_info_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a bond market data source.
    pub fn with_bond_market_source<S: BondMarketSource + 'static>(mut self, source: S) -> Self {
        self.bond_market_sources.push(Arc::new(source));
        self.bond_market_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a combined ETF info and market source.
    pub fn with_fund_source<S: FundInfoSource + FundMarketSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self.fund_info_sources.push(Arc::new(source.clone()));
        self.fund_market_sources.push(Arc::new(source));
        self.fund_info_sources.sort_by_key(|s| s.priority());
        self.fund_market_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a combined bond info and market source.
    pub fn with_bond_source<S: BondInfoSource + BondMarketSource + Clone + 'static>(
        mut self,
        source: S,
    ) -> Self {
        self.bond_info_sources.push(Arc::new(source.clone()));
        self.bond_market_sources.push(Arc::new(source));
        self.bond_info_sources.sort_by_key(|s| s.priority());
        self.bond_market_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Adds a news source.
    pub fn with_news_source<S: NewsSource + 'static>(mut self, source: S) -> Self {
        self.news_sources.push(Arc::new(source));
        self.news_sources.sort_by_key(|s| s.priority());
        self
    }

    /// Returns the number of registered market sources.
    pub fn market_source_count(&self) -> usize {
        self.market_sources.len()
    }

    /// Returns the number of registered info sources.
    pub fn info_source_count(&self) -> usize {
        self.info_sources.len()
    }

    /// Fetches historical K-line market data.
    pub async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        info!("Fetching market data for {} ({:?})", stock_code, k_type);

        for source in &self.market_sources {
            debug!("Trying source: {}", source.name());

            if !source.is_available().await {
                debug!("Source {} is not available, skipping", source.name());
                continue;
            }

            match source
                .get_market(stock_code, start_date, end_date, k_type)
                .await
            {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} records from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => {
                    debug!("Source {} returned empty data, trying next", source.name());
                    continue;
                }
                Err(e) => {
                    warn!("Source {} failed: {}", source.name(), e);
                    if !e.is_recoverable() {
                        return Err(e);
                    }
                    continue;
                }
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches real-time market quotes.
    pub async fn get_market_current(
        &self,
        stock_codes: &[&str],
    ) -> DataResult<Vec<CurrentMarketData>> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            "Fetching current market data for {} stocks",
            stock_codes.len()
        );

        for source in &self.market_sources {
            debug!("Trying source: {}", source.name());

            if !source.is_available().await {
                continue;
            }

            match source.get_market_current(stock_codes).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} current records from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) => {
                    warn!("Source {} failed: {}", source.name(), e);
                    if !e.is_recoverable() {
                        return Err(e);
                    }
                    continue;
                }
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches intraday minute-level data.
    pub async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        info!("Fetching minute data for {}", stock_code);

        for source in &self.market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_market_min(stock_code).await {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches order book data.
    pub async fn get_order_book(&self, stock_code: &str) -> DataResult<OrderBookData> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        for source in &self.market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_order_book(stock_code).await {
                Ok(data) => return Ok(data),
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches tick-by-tick trade data.
    pub async fn get_ticks(&self, stock_code: &str) -> DataResult<Vec<TickData>> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        for source in &self.market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_ticks(stock_code).await {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches all available stock codes.
    pub async fn get_all_codes(&self, limit: Option<usize>) -> DataResult<Vec<StockCode>> {
        if self.info_sources.is_empty() {
            return Err(DataError::custom("No info sources configured"));
        }

        info!("Fetching all stock codes");

        for source in &self.info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_codes(limit).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} stock codes from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches detailed stock information.
    pub async fn get_stock_info(&self, stock_code: &str) -> DataResult<StockInfo> {
        if self.info_sources.is_empty() {
            return Err(DataError::custom("No info sources configured"));
        }

        for source in &self.info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_stock_info(stock_code).await {
                Ok(info) => return Ok(info),
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches all available ETF codes.
    pub async fn get_all_etf_codes(&self, limit: Option<usize>) -> DataResult<Vec<ETFCode>> {
        if self.fund_info_sources.is_empty() {
            return Err(DataError::custom("No fund info sources configured"));
        }

        info!("Fetching all ETF codes");

        for source in &self.fund_info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_etf_codes(limit).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} ETF codes from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches historical ETF K-line market data.
    pub async fn get_etf_market(
        &self,
        fund_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        if self.fund_market_sources.is_empty() {
            return Err(DataError::custom("No fund market sources configured"));
        }

        info!("Fetching ETF market data for {}", fund_code);

        for source in &self.fund_market_sources {
            if !source.is_available().await {
                continue;
            }

            match source
                .get_etf_market(fund_code, start_date, end_date, k_type)
                .await
            {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches real-time ETF quotes.
    pub async fn get_etf_current(&self, fund_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        if self.fund_market_sources.is_empty() {
            return Err(DataError::custom("No fund market sources configured"));
        }

        if fund_codes.is_empty() {
            return Ok(Vec::new());
        }

        for source in &self.fund_market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_etf_current(fund_codes).await {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches intraday ETF minute-level data.
    pub async fn get_etf_min(&self, fund_code: &str) -> DataResult<Vec<ETFMinuteData>> {
        if self.fund_market_sources.is_empty() {
            return Err(DataError::custom("No fund market sources configured"));
        }

        for source in &self.fund_market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_etf_min(fund_code).await {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches all available convertible bond codes.
    pub async fn get_all_bond_codes(&self, limit: Option<usize>) -> DataResult<Vec<ConvertibleBondCode>> {
        if self.bond_info_sources.is_empty() {
            return Err(DataError::custom("No bond info sources configured"));
        }

        info!("Fetching all bond codes");

        for source in &self.bond_info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_bond_codes(limit).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} bond codes from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches real-time bond quotes.
    pub async fn get_bond_current(
        &self,
        bond_codes: Option<&[&str]>,
    ) -> DataResult<Vec<BondCurrentData>> {
        if self.bond_market_sources.is_empty() {
            return Err(DataError::custom("No bond market sources configured"));
        }

        info!("Fetching bond current data");

        for source in &self.bond_market_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_bond_current(bond_codes).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} bond records from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches news articles by category.
    pub async fn get_news(
        &self,
        category: NewsCategory,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        if self.news_sources.is_empty() {
            return Err(DataError::custom("No news sources configured"));
        }

        info!("Fetching news: category={:?}, page={}", category, page);

        for source in &self.news_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_news(category, page, limit).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully fetched {} news articles from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Searches news articles by keyword.
    pub async fn search_news(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> DataResult<Vec<NewsArticle>> {
        if self.news_sources.is_empty() {
            return Err(DataError::custom("No news sources configured"));
        }

        info!("Searching news: keyword={}, page={}", keyword, page);

        for source in &self.news_sources {
            if !source.is_available().await {
                continue;
            }

            match source.search_news(keyword, page, limit).await {
                Ok(data) if !data.is_empty() => {
                    info!(
                        "Successfully found {} news articles from {}",
                        data.len(),
                        source.name()
                    );
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    /// Fetches full news content by ID.
    pub async fn get_news_content(&self, news_id: &str) -> DataResult<NewsContent> {
        if self.news_sources.is_empty() {
            return Err(DataError::custom("No news sources configured"));
        }

        info!("Fetching news content: id={}", news_id);

        for source in &self.news_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_news_content(news_id).await {
                Ok(content) => {
                    info!("Successfully fetched news content from {}", source.name());
                    return Ok(content);
                }
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }
}

impl Default for DataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DataClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataClient")
            .field("market_sources", &self.market_sources.len())
            .field("info_sources", &self.info_sources.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = DataClient::new();
        assert_eq!(client.market_source_count(), 0);
        assert_eq!(client.info_source_count(), 0);
    }

    #[test]
    fn test_client_default() {
        let client = DataClient::default();
        assert_eq!(client.market_source_count(), 0);
    }
}
