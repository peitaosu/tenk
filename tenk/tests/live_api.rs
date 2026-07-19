use tenk::{
    BoardCrosswalkKind, ClientBuilder, FinancialReportKind, KLineType, LimitPoolKind,
    NewsCategory, OptionExchange, SourceKind,
};

fn live_enabled() -> bool {
    std::env::var("TENK_LIVE_TEST").as_deref() == Ok("1")
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
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let data = client
        .get_market("600519", Some("20250101"), Some("20250131"), KLineType::Daily)
        .await
        .unwrap();
    assert!(!data.is_empty(), "expected kline rows");
    assert!(data[0].close > 0.0);
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
    let client = ClientBuilder::with_sources(&[SourceKind::Eastmoney])
        .build()
        .unwrap();
    let pool = client
        .get_limit_pool(LimitPoolKind::LimitUp, None, Some(5))
        .await
        .unwrap();
    assert!(!pool.is_empty());
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
    assert!(data[0].price > 0.0);
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
    let boards = client.get_concept_boards(Some(5)).await.unwrap();
    assert!(boards.len() >= 3);
    assert!(boards[0].board_code.starts_with("885"));
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
