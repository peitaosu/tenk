use async_trait::async_trait;

use crate::data::{
    CurrentMarketData, DerivativesQuote, ETFCurrentData, ETFMarketData, IndexCode, KLineType,
    MarketData, MinuteData, StockCode, TvChartOptions,
};
use crate::error::{DataError, DataResult};
use crate::traits::{
    FundMarketSource, FuturesSource, GlobalMarketSource, IndexMarketSource, StockMarketSource,
};

use super::convert::{
    chart_options_for_kline, normalize_market_symbol, to_hk_tv_symbol, to_us_tv_symbol,
    tv_bar_to_minute, tv_quote_to_current,
};
use super::TradingViewSource;

const DEFAULT_KLINE_LIMIT: usize = 100;

#[async_trait]
impl StockMarketSource for TradingViewSource {
    async fn get_market(
        &self,
        stock_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        let options = chart_options_for_kline(k_type, start_date, end_date, DEFAULT_KLINE_LIMIT);
        let (bars, _) = self.chart(stock_code, &options).await?;
        let symbol = normalize_market_symbol(stock_code);
        Ok(bars
            .iter()
            .map(|bar| bar.to_market_data(&symbol))
            .collect())
    }

    async fn get_market_symbol(
        &self,
        symbol: &StockCode,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        self.get_market(&symbol.tv_symbol(), start_date, end_date, k_type)
            .await
    }

    async fn get_market_min_symbol(&self, symbol: &StockCode) -> DataResult<Vec<MinuteData>> {
        self.get_market_min(&symbol.tv_symbol()).await
    }

    async fn get_market_current_symbols(
        &self,
        symbols: &[StockCode],
    ) -> DataResult<Vec<CurrentMarketData>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }
        let tv_symbols: Vec<String> = symbols.iter().map(|symbol| symbol.tv_symbol()).collect();
        let refs: Vec<&str> = tv_symbols.iter().map(String::as_str).collect();
        self.get_market_current(&refs).await
    }

    async fn get_market_current(&self, stock_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let quotes = self.quote(stock_codes).await?;
        Ok(quotes.iter().map(tv_quote_to_current).collect())
    }

    async fn get_market_min(&self, stock_code: &str) -> DataResult<Vec<MinuteData>> {
        let options = TvChartOptions {
            timeframe: crate::data::TvTimeFrame::Min1,
            range: 240,
            ..TvChartOptions::default()
        };
        let (bars, _) = self.chart(stock_code, &options).await?;
        let symbol = normalize_market_symbol(stock_code);
        Ok(bars.iter().map(|bar| tv_bar_to_minute(bar, &symbol)).collect())
    }
}

#[async_trait]
impl FundMarketSource for TradingViewSource {
    async fn get_etf_market(
        &self,
        etf_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<ETFMarketData>> {
        let data = self
            .get_market(etf_code, start_date, end_date, k_type)
            .await?;
        Ok(data
            .into_iter()
            .map(|bar| ETFMarketData {
                fund_code: bar.stock_code,
                trade_time: bar.trade_time,
                trade_date: bar.trade_date,
                open: bar.open,
                close: bar.close,
                high: bar.high,
                low: bar.low,
                volume: bar.volume,
                amount: bar.amount,
                change: Some(bar.change),
                change_pct: Some(bar.change_pct),
            })
            .collect())
    }

    async fn get_etf_current(&self, etf_codes: &[&str]) -> DataResult<Vec<ETFCurrentData>> {
        let quotes = self.get_market_current(etf_codes).await?;
        Ok(quotes
            .into_iter()
            .map(|quote| ETFCurrentData {
                fund_code: quote.stock_code,
                short_name: quote.short_name,
                price: quote.price,
                change: Some(quote.change),
                change_pct: Some(quote.change_pct),
                volume: quote.volume,
                amount: quote.amount,
                open: quote.open,
                high: quote.high,
                low: quote.low,
            })
            .collect())
    }

    async fn get_etf_min(&self, _etf_code: &str) -> DataResult<Vec<crate::data::ETFMinuteData>> {
        Err(DataError::not_supported("get_etf_min"))
    }
}

#[async_trait]
impl IndexMarketSource for TradingViewSource {
    async fn get_index_current(&self, index_codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        self.get_market_current(index_codes).await
    }

    async fn get_index_market(
        &self,
        index_code: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        self.get_market(index_code, start_date, end_date, k_type)
            .await
    }

    async fn get_index_list(&self, _limit: Option<usize>) -> DataResult<Vec<IndexCode>> {
        Err(DataError::not_supported("get_index_list"))
    }
}

#[async_trait]
impl GlobalMarketSource for TradingViewSource {
    async fn get_hk_current(&self, codes: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let symbols: Vec<String> = codes.iter().map(|code| to_hk_tv_symbol(code)).collect();
        let refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
        self.get_market_current(&refs).await
    }

    async fn get_us_current(&self, symbols: &[&str]) -> DataResult<Vec<CurrentMarketData>> {
        let mapped: Vec<String> = symbols.iter().map(|s| to_us_tv_symbol(s)).collect();
        let refs: Vec<&str> = mapped.iter().map(String::as_str).collect();
        self.get_market_current(&refs).await
    }
}

#[async_trait]
impl FuturesSource for TradingViewSource {
    async fn get_futures_current(&self, secids: &[&str]) -> DataResult<Vec<DerivativesQuote>> {
        let quotes = self.get_market_current(secids).await?;
        Ok(quotes
            .into_iter()
            .map(|quote| DerivativesQuote {
                contract_code: quote.stock_code.clone(),
                contract_name: quote.short_name,
                secid: quote.stock_code,
                price: quote.price,
                change: quote.change,
                change_pct: quote.change_pct,
                volume: quote.volume,
                amount: quote.amount,
                open: quote.open,
                high: quote.high,
                low: quote.low,
                pre_close: quote.pre_close,
                open_interest: None,
                trade_date: None,
            })
            .collect())
    }

    async fn get_futures_market(
        &self,
        secid: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        k_type: KLineType,
    ) -> DataResult<Vec<MarketData>> {
        self.get_market(secid, start_date, end_date, k_type).await
    }
}
