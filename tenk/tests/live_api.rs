use tenk::{
    BoardCrosswalkKind, ClientBuilder, FinancialReportKind, KLineType, LimitPoolKind,
    NewsCategory, OptionExchange, SourceKind,
};

fn live_enabled() -> bool {
    std::env::var("TENK_LIVE_TEST").as_deref() == Ok("1")
}

fn eastmoney_client() -> tenk::DataClient {
    let mut builder = ClientBuilder::with_sources(&[SourceKind::Eastmoney]);
    if let Ok(proxy) = std::env::var("TENK_TV_PROXY") {
        builder = builder.with_proxy(proxy);
    }
    builder.build().unwrap()
}

macro_rules! live_test {
    ($name:ident, $body:block) => {
        #[tokio::test]
        async fn $name() {
            if !live_enabled() {
                return;
            }
            $body
        }
    };
}

live_test!(live_eastmoney_stock_current, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let data = client.get_market_current(&["600519"]).await.unwrap();
    assert!(!data.is_empty(), "expected stock quote");
    assert!(data[0].price > 0.0, "price should be positive");
    assert!(!data[0].short_name.is_empty(), "name should not be empty");
});

live_test!(live_eastmoney_stock_kline, {
    let client = eastmoney_client();
    match client
        .get_market("600519", Some("20250101"), Some("20250131"), KLineType::Daily)
        .await
    {
        Ok(data) => {
            assert!(!data.is_empty(), "expected kline rows");
            assert!(data[0].close > 0.0);
        }
        Err(tenk::DataError::NoDataAvailable) => {}
        Err(error) => panic!("unexpected error: {error}"),
    }
});

live_test!(live_eastmoney_stock_minute_day, {
    use tenk::StockCode;

    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let symbol = StockCode::with_inferred_exchange("600519");
    let data = client.get_market_min_for(&symbol).await.unwrap();
    assert!(!data.is_empty(), "expected intraday minute rows");
    assert!(data[0].price > 0.0);
});

live_test!(live_eastmoney_stock_minute_five_days, {
    use tenk::StockCode;

    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let symbol = StockCode::with_inferred_exchange("600519");
    let data = client.get_market_min_days_for(&symbol, 5).await.unwrap();
    assert!(!data.is_empty(), "expected intraday minute rows");
    let mut dates = data
        .iter()
        .map(|minute| tenk::cn_market_date(minute.trade_time))
        .collect::<Vec<_>>();
    dates.sort();
    dates.dedup();
    assert!(
        !dates.is_empty(),
        "expected at least one trading day of minute data"
    );
});

live_test!(live_eastmoney_index_current, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let data = client.get_index_current(&["000001"]).await.unwrap();
    assert!(!data.is_empty());
    assert!(data[0].price > 0.0);
});

live_test!(live_eastmoney_industry_boards, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let boards = client.get_industry_boards(Some(5)).await.unwrap();
    assert!(boards.len() >= 3);
    assert!(boards[0].board_code.starts_with("BK"));
});

live_test!(live_eastmoney_board_constituents, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let boards = client.get_industry_boards(Some(1)).await.unwrap();
    let code = &boards[0].board_code;
    let members = client.get_board_constituents(code, Some(10)).await.unwrap();
    assert!(!members.is_empty(), "expected board members for {code}");
    assert_eq!(members[0].stock_code.len(), 6);
});

live_test!(live_eastmoney_limit_pool, {
    let client = eastmoney_client();
    match client
        .get_limit_pool(LimitPoolKind::LimitUp, None, Some(5))
        .await
    {
        Ok(pool) => assert!(!pool.is_empty()),
        Err(tenk::DataError::NoDataAvailable) => {}
        Err(error) => panic!("unexpected error: {error}"),
    }
});

live_test!(live_eastmoney_macro_cpi, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let records = client.get_macro_cpi(Some(3)).await.unwrap();
    assert!(!records.is_empty());
});

live_test!(live_eastmoney_futures, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let list = client.get_futures_list(Some(5)).await.unwrap();
    assert!(!list.is_empty());
    let secid = &list[0].secid;
    let quotes = client.get_futures_current(&[secid.as_str()]).await.unwrap();
    assert!(!quotes.is_empty());
    assert!(quotes[0].price > 0.0);
});

live_test!(live_eastmoney_options, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let list = client
        .get_options_list(OptionExchange::Sse, Some(5))
        .await
        .unwrap();
    assert!(!list.is_empty());
});

live_test!(live_eastmoney_financials, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let records = client
        .get_financial_statement("600519", FinancialReportKind::IncomeStatement, Some(2))
        .await
        .unwrap();
    assert!(!records.is_empty());
});

live_test!(live_eastmoney_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let articles = client.get_news(NewsCategory::Finance, 1, 5).await.unwrap();
    assert!(!articles.is_empty());
    assert!(!articles[0].title.is_empty());
    let content = client.get_news_content(&articles[0].id).await.unwrap();
    assert!(!content.body_text.is_empty());
});

live_test!(live_eastmoney_search_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let articles = client.search_news("茅台", 1, 5).await.unwrap();
    assert!(!articles.is_empty());
});

live_test!(live_sina_stock_current, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client.get_market_current(&["600519"]).await.unwrap();
    assert!(!data.is_empty());
    assert!(data[0].price >= 0.0);
});

live_test!(live_sina_index_current, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client.get_index_current(&["000001"]).await.unwrap();
    assert!(!data.is_empty());
    assert!(data[0].price > 1000.0);
});

live_test!(live_sina_futures, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let quotes = client.get_futures_current(&["ZN0"]).await.unwrap();
    assert!(!quotes.is_empty());
    assert!(quotes[0].price > 0.0);
    assert!(quotes[0].volume > 0);
});

live_test!(live_sina_stock_kline, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client
        .get_market("600519", Some("20250101"), Some("20250131"), KLineType::Daily)
        .await
        .unwrap();
    assert!(!data.is_empty());
});

live_test!(live_ths_stock_kline, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let data = client
        .get_market("600519", Some("20250101"), Some("20250131"), KLineType::Daily)
        .await
        .unwrap();
    assert!(!data.is_empty());
});

live_test!(live_ths_industry_boards, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let boards = client.get_industry_boards(Some(5)).await.unwrap();
    assert!(boards.len() >= 3);
    assert!(boards[0].board_code.starts_with("881"));
});

live_test!(live_ths_concept_boards, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    match client.get_concept_boards(Some(5)).await {
        Ok(boards) => {
            assert!(boards.len() >= 3);
            assert!(boards[0].board_code.starts_with("885"));
        }
        Err(tenk::DataError::NoDataAvailable) => {
            eprintln!("skip: THS concept boards unavailable");
        }
        Err(error) => panic!("unexpected error: {error}"),
    }
});

live_test!(live_ths_board_kline, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let boards = client.get_industry_boards(Some(1)).await.unwrap();
    let code = &boards[0].board_code;
    let data = client
        .get_board_market(code, Some("20260601"), Some("20260630"), KLineType::Daily)
        .await
        .unwrap();
    assert!(!data.is_empty(), "expected board kline for {code}");
});

live_test!(live_ths_board_constituents, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let boards = client.get_industry_boards(Some(1)).await.unwrap();
    let code = &boards[0].board_code;
    let members = client.get_board_constituents(code, Some(10)).await.unwrap();
    assert!(!members.is_empty(), "expected THS board members for {code}");
});

live_test!(live_ths_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let articles = client.get_news(NewsCategory::Finance, 1, 5).await.unwrap();
    assert!(!articles.is_empty());
    assert!(!articles[0].title.is_empty());
    let content = client.get_news_content(&articles[0].id).await.unwrap();
    assert!(!content.body_text.is_empty());
});

live_test!(live_ths_search_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let articles = client.search_news("银行", 1, 5).await.unwrap();
    assert!(!articles.is_empty());
});

live_test!(live_sina_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client
        .get_news(NewsCategory::Finance, 1, 5)
        .await
        .unwrap();
    assert!(!data.is_empty());
    let content = client.get_news_content(&data[0].id).await.unwrap();
    assert!(!content.body_text.is_empty());
});

live_test!(live_sina_search_news, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client.search_news("600519", 1, 5).await.unwrap();
    assert!(!data.is_empty());
});

live_test!(live_sina_research_reports, {
    let client = ClientBuilder::with_sources(&[SourceKind::Sina])
        .build()
        .unwrap();
    let data = client
        .get_research_reports(Some("600519"), 1, Some(5))
        .await
        .unwrap();
    assert!(!data.is_empty());
    assert!(!data[0].title.is_empty());
});

live_test!(live_ths_research_reports, {
    let client = ClientBuilder::with_sources(&[SourceKind::Ths])
        .build()
        .unwrap();
    let data = client
        .get_research_reports(Some("600519"), 1, Some(5))
        .await
        .unwrap();
    assert!(!data.is_empty());
    assert!(!data[0].title.is_empty());
    assert!(!data[0].institution.is_empty());
});

live_test!(live_board_crosswalk_industry, {
    let client = ClientBuilder::new().build().unwrap();
    let crosswalk = client
        .resolve_board_crosswalk(BoardCrosswalkKind::Industry, Some(100))
        .await
        .unwrap();
    assert!(crosswalk.len() >= 10);
    let matched = crosswalk
        .iter()
        .filter(|item| item.eastmoney_code.is_some() && item.ths_code.is_some())
        .count();
    assert!(matched >= 1, "expected at least one name-matched industry board");
    let resolved = client
        .resolve_ths_board_for_eastmoney("BK0428", Some(30))
        .await
        .unwrap();
    assert!(
        resolved.as_ref().and_then(|item| item.ths_code.as_deref()) == Some("881145"),
        "expected constituent-based mapping for 电力"
    );
});

live_test!(live_board_crosswalk_concept, {
    let client = ClientBuilder::new().build().unwrap();
    let crosswalk = client
        .resolve_board_crosswalk(BoardCrosswalkKind::Concept, Some(50))
        .await
        .unwrap();
    assert!(crosswalk.len() >= 10);
});

fn tv_live_enabled() -> bool {
    live_enabled() && std::env::var("TENK_TV_PROXY").is_ok()
}

macro_rules! tv_live_test {
    ($name:ident, $body:block) => {
        #[tokio::test]
        async fn $name() {
            if !tv_live_enabled() {
                return;
            }
            $body
        }
    };
}

fn tv_client() -> tenk::DataClient {
    let proxy = std::env::var("TENK_TV_PROXY").ok();
    let mut builder = ClientBuilder::with_sources(&[SourceKind::Tradingview]);
    if let Some(proxy_url) = proxy {
        builder = builder.with_proxy(proxy_url);
    }
    builder.build().unwrap()
}

tv_live_test!(live_tradingview_search, {
    let client = tv_client();
    let data = client.search_symbols("600519", None, 0).await.unwrap();
    assert!(!data.is_empty());
    assert!(data.iter().any(|item| item.symbol == "600519"));
});

tv_live_test!(live_tradingview_ta, {
    let client = tv_client();
    let data = client.get_technical_analysis("600519").await.unwrap();
    assert_eq!(data.symbol, "SSE:600519");
    assert!(!data.periods.is_empty());
});

tv_live_test!(live_tradingview_analyst, {
    let client = tv_client();
    let data = client.get_analyst("AAPL").await.unwrap();
    assert_eq!(data.symbol, "NASDAQ:AAPL");
    assert!(data.ratings.total > 0);
    assert!(data.price_targets.average.is_some());
});

tv_live_test!(live_tradingview_quote, {
    let client = tv_client();
    let data = client.get_market_current(&["AAPL"]).await.unwrap();
    assert!(!data.is_empty());
    assert!(!data[0].short_name.is_empty());
});

tv_live_test!(live_tradingview_screener, {
    let client = tv_client();
    let request = tenk::TvScreenerRequest {
        market: "china".into(),
        columns: vec!["name".into(), "close".into(), "change".into()],
        filters: Vec::new(),
        sort_by: "change".into(),
        sort_order: "desc".into(),
        range_start: 0,
        range_end: 5,
        preset: "all_stocks".into(),
    };
    let data = client.run_screener(&request).await.unwrap();
    assert!(data.total_count > 0);
    assert!(!data.rows.is_empty());
});

tv_live_test!(live_tradingview_calendar, {
    let client = tv_client();
    let data = client
        .get_economic_calendar("2026-07-01", "2026-07-31", "CN,US")
        .await
        .unwrap();
    assert!(!data.is_empty());
});

tv_live_test!(live_tradingview_news, {
    let client = tv_client();
    let data = client
        .get_news(NewsCategory::Stock, 1, 5)
        .await
        .unwrap();
    assert!(!data.is_empty());
    let content = client.get_news_content(&data[0].id).await.unwrap();
    assert!(!content.body_text.is_empty());
});

tv_live_test!(live_tradingview_search_news, {
    let client = tv_client();
    let by_symbol = client.search_news("AAPL", 1, 5).await.unwrap();
    assert!(!by_symbol.is_empty());
    match client.search_news("Apple", 1, 5).await {
        Ok(by_text) => assert!(!by_text.is_empty()),
        Err(tenk::DataError::NoDataAvailable) => {}
        Err(error) => panic!("unexpected error: {error}"),
    }
});

tv_live_test!(live_tradingview_chart, {
    let client = tv_client();
    let options = tenk::TvChartOptions {
        range: 5,
        ..tenk::TvChartOptions::default()
    };
    let data = client
        .get_market("AAPL", None, None, KLineType::Daily)
        .await
        .unwrap();
    assert!(!data.is_empty());
    let _ = options;
});

tv_live_test!(live_tradingview_indicator_series, {
    let client = tv_client();
    let options = tenk::TvChartOptions {
        range: 5,
        ..tenk::TvChartOptions::default()
    };
    let series = client
        .get_indicator_series("AAPL", "STD;RSI", "last", &options)
        .await
        .unwrap();
    assert_eq!(series.symbol, "NASDAQ:AAPL");
    assert!(!series.points.is_empty());
});

tv_live_test!(live_tradingview_strategy, {
    let client = tv_client();
    let options = tenk::TvChartOptions {
        range: 100,
        ..tenk::TvChartOptions::default()
    };
    match client
        .get_strategy_report("AAPL", "STD;RSI%1Strategy", "last", &options)
        .await
    {
        Ok(report) => {
            assert!(
                report.performance.total_trades.is_some()
                    || report.performance.net_profit.is_some()
                    || !report.trades.is_empty()
            );
        }
        Err(tenk::DataError::NoDataAvailable) => {
            eprintln!("skip: TradingView strategy report unavailable");
        }
        Err(error) if error.to_string().contains("study error") => {
            eprintln!("skip: TradingView strategy study error: {error}");
        }
        Err(error) => panic!("unexpected error: {error}"),
    }
});
