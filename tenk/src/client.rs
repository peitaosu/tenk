//! Data client with multi-source fallback.

use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::data::{
    BondCurrentData, ConvertibleBondCode, CurrentMarketData, ETFCode, ETFCurrentData,
    ETFMarketData, ETFMinuteData, KLineType, MarketData, MinuteData, OrderBookData, StockCode,
    StockInfo, TickData,
};
use crate::error::{DataError, DataResult};
use crate::traits::{
    BondInfoSource, BondMarketSource, FundInfoSource, FundMarketSource, StockInfoSource,
    StockMarketSource,
};

/// Data client.
pub struct DataClient {
    market_sources: Vec<Arc<dyn StockMarketSource>>,
    info_sources: Vec<Arc<dyn StockInfoSource>>,
    fund_info_sources: Vec<Arc<dyn FundInfoSource>>,
    fund_market_sources: Vec<Arc<dyn FundMarketSource>>,
    bond_info_sources: Vec<Arc<dyn BondInfoSource>>,
    bond_market_sources: Vec<Arc<dyn BondMarketSource>>,
}

impl DataClient {
    pub fn new() -> Self {
        Self {
            market_sources: Vec::new(),
            info_sources: Vec::new(),
            fund_info_sources: Vec::new(),
            fund_market_sources: Vec::new(),
            bond_info_sources: Vec::new(),
            bond_market_sources: Vec::new(),
        }
    }

    pub fn with_market_source<S: StockMarketSource + 'static>(mut self, source: S) -> Self {
        self.market_sources.push(Arc::new(source));
        self.market_sources.sort_by_key(|s| s.priority());
        self
    }

    pub fn with_info_source<S: StockInfoSource + 'static>(mut self, source: S) -> Self {
        self.info_sources.push(Arc::new(source));
        self.info_sources.sort_by_key(|s| s.priority());
        self
    }

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

    pub fn with_fund_info_source<S: FundInfoSource + 'static>(mut self, source: S) -> Self {
        self.fund_info_sources.push(Arc::new(source));
        self.fund_info_sources.sort_by_key(|s| s.priority());
        self
    }

    pub fn with_fund_market_source<S: FundMarketSource + 'static>(mut self, source: S) -> Self {
        self.fund_market_sources.push(Arc::new(source));
        self.fund_market_sources.sort_by_key(|s| s.priority());
        self
    }

    pub fn with_bond_info_source<S: BondInfoSource + 'static>(mut self, source: S) -> Self {
        self.bond_info_sources.push(Arc::new(source));
        self.bond_info_sources.sort_by_key(|s| s.priority());
        self
    }

    pub fn with_bond_market_source<S: BondMarketSource + 'static>(mut self, source: S) -> Self {
        self.bond_market_sources.push(Arc::new(source));
        self.bond_market_sources.sort_by_key(|s| s.priority());
        self
    }

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

    pub fn market_source_count(&self) -> usize {
        self.market_sources.len()
    }

    pub fn info_source_count(&self) -> usize {
        self.info_sources.len()
    }

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

            match source.get_market(stock_code, start_date, end_date, k_type).await {
                Ok(data) if !data.is_empty() => {
                    info!("Successfully fetched {} records from {}", data.len(), source.name());
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

    pub async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        if self.market_sources.is_empty() {
            return Err(DataError::custom("No market sources configured"));
        }

        if stock_codes.is_empty() {
            return Ok(Vec::new());
        }

        info!("Fetching current market data for {} stocks", stock_codes.len());

        for source in &self.market_sources {
            debug!("Trying source: {}", source.name());

            if !source.is_available().await {
                continue;
            }

            match source.get_market_current(stock_codes).await {
                Ok(data) if !data.is_empty() => {
                    info!("Successfully fetched {} current records from {}", data.len(), source.name());
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

    pub async fn get_all_codes(&self) -> DataResult<Vec<StockCode>> {
        if self.info_sources.is_empty() {
            return Err(DataError::custom("No info sources configured"));
        }

        info!("Fetching all stock codes");

        for source in &self.info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_codes().await {
                Ok(data) if !data.is_empty() => {
                    info!("Successfully fetched {} stock codes from {}", data.len(), source.name());
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

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

    pub async fn get_all_etf_codes(&self) -> DataResult<Vec<ETFCode>> {
        if self.fund_info_sources.is_empty() {
            return Err(DataError::custom("No fund info sources configured"));
        }

        info!("Fetching all ETF codes");

        for source in &self.fund_info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_etf_codes().await {
                Ok(data) if !data.is_empty() => {
                    info!("Successfully fetched {} ETF codes from {}", data.len(), source.name());
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

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

            match source.get_etf_market(fund_code, start_date, end_date, k_type).await {
                Ok(data) if !data.is_empty() => return Ok(data),
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

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

    pub async fn get_all_bond_codes(&self) -> DataResult<Vec<ConvertibleBondCode>> {
        if self.bond_info_sources.is_empty() {
            return Err(DataError::custom("No bond info sources configured"));
        }

        info!("Fetching all bond codes");

        for source in &self.bond_info_sources {
            if !source.is_available().await {
                continue;
            }

            match source.get_all_bond_codes().await {
                Ok(data) if !data.is_empty() => {
                    info!("Successfully fetched {} bond codes from {}", data.len(), source.name());
                    return Ok(data);
                }
                Ok(_) => continue,
                Err(e) if e.is_recoverable() => continue,
                Err(e) => return Err(e),
            }
        }

        Err(DataError::NoDataAvailable)
    }

    pub async fn get_bond_current(&self, bond_codes: Option<&[&str]>) -> DataResult<Vec<BondCurrentData>> {
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
                    info!("Successfully fetched {} bond records from {}", data.len(), source.name());
                    return Ok(data);
                }
                Ok(_) => continue,
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
