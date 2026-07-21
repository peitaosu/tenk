use std::sync::Arc;

use tenk::{
    CurrentMarketData, DataClient, KLineType, MarketData, MinuteData, NewsContent,
    StockCode, StockSearchHit, StockValuation,
};
use tokio::sync::mpsc;

use super::feed::{FeedData, FeedKind, FeedScope, FEED_PAGE_SIZE};
use super::kline::{KlineView, TimelineScope};

const INTRADAY_DAY_NDAYS: u32 = 1;
const INTRADAY_FIVE_DAY_NDAYS: u32 = 5;
const STOCK_SEARCH_LIMIT: usize = 12;
const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);
const INTRADAY_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn intraday_ndays(scope: TimelineScope) -> u32 {
    match scope {
        TimelineScope::Day => INTRADAY_DAY_NDAYS,
        TimelineScope::FiveDays => INTRADAY_FIVE_DAY_NDAYS,
    }
}

fn intraday_fetch_timeout(ndays: u32) -> std::time::Duration {
    if ndays > 1 {
        INTRADAY_FETCH_TIMEOUT
    } else {
        FETCH_TIMEOUT
    }
}

pub enum FetchMsg {
    Watchlist(Vec<CurrentMarketData>),
    Detail {
        fetch_id: u64,
        symbol: String,
        quote: Option<CurrentMarketData>,
        valuation: Option<StockValuation>,
    },
    Kline {
        fetch_id: u64,
        symbol: String,
        kline: Vec<MarketData>,
    },
    Intraday {
        fetch_id: u64,
        symbol: String,
        data: Vec<MinuteData>,
    },
    KlinePoll {
        symbol: String,
        kline: Vec<MarketData>,
    },
    IntradayPoll {
        symbol: String,
        data: Vec<MinuteData>,
    },
    Feed {
        index: usize,
        fetch_id: u64,
        data: FeedData,
    },
    NewsContent(NewsContent),
    StockSearch {
        fetch_id: u64,
        query: String,
        results: Vec<StockSearchHit>,
    },
    Error { fetch_id: u64, message: String },
}

fn enrich_symbol_from_quote(symbol: StockCode, quote: Option<&CurrentMarketData>) -> StockCode {
    let Some(quote) = quote else {
        return symbol;
    };
    if symbol.short_name.is_empty() && !quote.short_name.is_empty() {
        StockCode::new(
            symbol.stock_code,
            quote.short_name.clone(),
            symbol.exchange,
        )
    } else {
        symbol
    }
}

async fn fetch_feed(client: &DataClient, symbol: &StockCode, kind: FeedKind, page: u32) -> FeedData {
    match kind {
        FeedKind::SymbolNews => {
            let articles = client
                .search_news_for_symbol(symbol, page, FEED_PAGE_SIZE)
                .await
                .unwrap_or_default();
            if !articles.is_empty() {
                return FeedData::Articles(articles);
            }
            if !symbol.short_name.is_empty() {
                let batch = client
                    .search_news(&symbol.short_name, page, FEED_PAGE_SIZE)
                    .await
                    .unwrap_or_default();
                if !batch.is_empty() {
                    return FeedData::Articles(batch);
                }
            }
            let batch = client
                .search_news(&symbol.stock_code, page, FEED_PAGE_SIZE)
                .await
                .unwrap_or_default();
            FeedData::Articles(batch)
        }
        FeedKind::ResearchReports => {
            let reports = client
                .get_research_reports(
                    Some(&symbol.stock_code),
                    page,
                    Some(FEED_PAGE_SIZE as usize),
                )
                .await
                .unwrap_or_default();
            FeedData::Reports(reports)
        }
        FeedKind::MarketNews(category) => {
            let articles = client
                .get_news(category, page, FEED_PAGE_SIZE)
                .await
                .unwrap_or_default();
            FeedData::Articles(articles)
        }
        FeedKind::InstitutionalResearch => {
            let rows = client
                .get_institutional_research(page, Some(FEED_PAGE_SIZE as usize))
                .await
                .unwrap_or_default();
            FeedData::Institutional(rows)
        }
        FeedKind::TechnicalAnalysis => client
            .get_technical_analysis(&symbol.stock_code)
            .await
            .map(FeedData::TechnicalAnalysis)
            .unwrap_or(FeedData::Empty),
        FeedKind::AnalystConsensus => client
            .get_analyst(&symbol.stock_code)
            .await
            .map(FeedData::Analyst)
            .unwrap_or(FeedData::Empty),
    }
}

pub fn spawn_watchlist(
    client: Arc<DataClient>,
    symbols: Vec<StockCode>,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        if symbols.is_empty() {
            return;
        }
        let result = tokio::time::timeout(FETCH_TIMEOUT, client.get_market_current_for(&symbols)).await;
        match result {
            Ok(Ok(data)) => {
                let _ = tx.send(FetchMsg::Watchlist(data));
            }
            Ok(Err(e)) => {
                let _ = tx.send(FetchMsg::Error {
                    fetch_id: 0,
                    message: e.to_string(),
                });
            }
            Err(_) => {
                let _ = tx.send(FetchMsg::Error {
                    fetch_id: 0,
                    message: "request timed out".to_string(),
                });
            }
        }
    });
}

pub fn spawn_symbol(
    client: Arc<DataClient>,
    symbol: StockCode,
    kline_type: KLineType,
    fetch_id: u64,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        let code = symbol.stock_code.clone();
        let (quote, valuation) = tokio::join!(
            async {
                tokio::time::timeout(
                    FETCH_TIMEOUT,
                    client.get_market_current_for(&[symbol.clone()]),
                )
                .await
                .ok()
                .and_then(Result::ok)
                .and_then(|mut quotes| quotes.pop())
            },
            async {
                tokio::time::timeout(FETCH_TIMEOUT, client.get_valuation_for(&symbol))
                    .await
                    .ok()
                    .and_then(Result::ok)
            },
        );
        let symbol = enrich_symbol_from_quote(symbol, quote.as_ref());
        let _ = tx.send(FetchMsg::Detail {
            fetch_id,
            symbol: code.clone(),
            quote: quote.clone(),
            valuation,
        });

        let (kline, intraday) = tokio::join!(
            async {
                tokio::time::timeout(
                    FETCH_TIMEOUT,
                    client.get_market_for(&symbol, None, None, kline_type),
                )
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
            },
            async {
                let ndays = intraday_ndays(TimelineScope::Day);
                tokio::time::timeout(
                    intraday_fetch_timeout(ndays),
                    client.get_market_min_days_for(&symbol, ndays),
                )
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default()
            },
        );

        let _ = tx.send(FetchMsg::Kline {
            fetch_id,
            symbol: code.clone(),
            kline,
        });
        let _ = tx.send(FetchMsg::Intraday {
            fetch_id,
            symbol: code,
            data: intraday,
        });
    });
}

pub fn spawn_symbol_feeds(
    client: Arc<DataClient>,
    symbol: StockCode,
    fetch_id: u64,
    panels: Vec<(usize, FeedKind, u32)>,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    if panels.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let quote = tokio::time::timeout(
            FETCH_TIMEOUT,
            client.get_market_current_for(&[symbol.clone()]),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|mut quotes| quotes.pop());
        let symbol = enrich_symbol_from_quote(symbol, quote.as_ref());
        let mut tasks = Vec::with_capacity(panels.len());
        for (index, kind, page) in panels {
            let client = client.clone();
            let symbol = symbol.clone();
            let tx = tx.clone();
            tasks.push(tokio::spawn(async move {
                let data = tokio::time::timeout(
                    FETCH_TIMEOUT,
                    fetch_feed(&client, &symbol, kind, page),
                )
                .await
                .ok()
                .unwrap_or(FeedData::Empty);
                let _ = tx.send(FetchMsg::Feed {
                    index,
                    fetch_id,
                    data,
                });
            }));
        }
        for task in tasks {
            let _ = task.await;
        }
    });
}

pub fn spawn_market_feeds(
    client: Arc<DataClient>,
    panels: Vec<(usize, FeedKind, u32)>,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    if panels.is_empty() {
        return;
    }
    tokio::spawn(async move {
        let mut tasks = Vec::with_capacity(panels.len());
        for (index, kind, page) in panels {
            let client = client.clone();
            let tx = tx.clone();
            tasks.push(tokio::spawn(async move {
                let data = tokio::time::timeout(
                    FETCH_TIMEOUT,
                    fetch_feed(&client, &StockCode::new(String::new(), String::new(), tenk::Exchange::Unknown), kind, page),
                )
                .await
                .ok()
                .unwrap_or(FeedData::Empty);
                let _ = tx.send(FetchMsg::Feed {
                    index,
                    fetch_id: 0,
                    data,
                });
            }));
        }
        for task in tasks {
            let _ = task.await;
        }
    });
}

pub fn spawn_feed(
    client: Arc<DataClient>,
    index: usize,
    kind: FeedKind,
    symbol: Option<StockCode>,
    page: u32,
    fetch_id: u64,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        let data = if kind.scope() == FeedScope::Symbol {
            let Some(mut symbol) = symbol else {
                let _ = tx.send(FetchMsg::Feed {
                    index,
                    fetch_id,
                    data: FeedData::Empty,
                });
                return;
            };
            if symbol.short_name.is_empty() {
                if let Ok(mut quotes) = client.get_market_current_for(&[symbol.clone()]).await {
                    if let Some(quote) = quotes.pop() {
                        symbol = enrich_symbol_from_quote(symbol, Some(&quote));
                    }
                }
            }
            tokio::time::timeout(FETCH_TIMEOUT, fetch_feed(&client, &symbol, kind, page))
                .await
                .ok()
                .unwrap_or(FeedData::Empty)
        } else {
            tokio::time::timeout(
                FETCH_TIMEOUT,
                fetch_feed(
                    &client,
                    &StockCode::new(String::new(), String::new(), tenk::Exchange::Unknown),
                    kind,
                    page,
                ),
            )
            .await
            .ok()
            .unwrap_or(FeedData::Empty)
        };
        let _ = tx.send(FetchMsg::Feed {
            index,
            fetch_id,
            data,
        });
    });
}

pub fn spawn_kline_poll(
    client: Arc<DataClient>,
    symbol: StockCode,
    kline_type: KLineType,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        let code = symbol.stock_code.clone();
        let kline = client
            .get_market_for(&symbol, None, None, kline_type)
            .await
            .unwrap_or_default();
        let _ = tx.send(FetchMsg::KlinePoll { symbol: code, kline });
    });
}

pub fn spawn_intraday_poll(
    client: Arc<DataClient>,
    symbol: StockCode,
    ndays: u32,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        let code = symbol.stock_code.clone();
        let data = tokio::time::timeout(
            intraday_fetch_timeout(ndays),
            client.get_market_min_days_for(&symbol, ndays),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
        let _ = tx.send(FetchMsg::IntradayPoll { symbol: code, data });
    });
}

pub fn spawn_kline_refresh(
    client: Arc<DataClient>,
    symbol: StockCode,
    view: KlineView,
    kline_type: KLineType,
    timeline_scope: TimelineScope,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    match view {
        KlineView::Timeline => {
            spawn_intraday_poll(client, symbol, intraday_ndays(timeline_scope), tx)
        }
        _ => spawn_kline_poll(client, symbol, kline_type, tx),
    }
}

pub fn spawn_news_content(
    client: Arc<DataClient>,
    id: String,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        match client.get_news_content(&id).await {
            Ok(content) => {
                let _ = tx.send(FetchMsg::NewsContent(content));
            }
            Err(e) => {
                let _ = tx.send(FetchMsg::Error {
                    fetch_id: 0,
                    message: e.to_string(),
                });
            }
        }
    });
}

pub fn spawn_stock_search(
    client: Arc<DataClient>,
    query: String,
    fetch_id: u64,
    tx: mpsc::UnboundedSender<FetchMsg>,
) {
    tokio::spawn(async move {
        let trimmed = query.trim().to_string();
        if trimmed.is_empty() {
            let _ = tx.send(FetchMsg::StockSearch {
                fetch_id,
                query: trimmed,
                results: Vec::new(),
            });
            return;
        }
        let results = client
            .search_stocks(&trimmed, STOCK_SEARCH_LIMIT)
            .await
            .unwrap_or_default();
        let _ = tx.send(FetchMsg::StockSearch {
            fetch_id,
            query: trimmed,
            results,
        });
    });
}
