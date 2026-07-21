use tenk::{TvChartOptions, TvScreenerRequest, TvTimeFrame};

pub fn chart_options(timeframe: &str, limit: usize) -> TvChartOptions {
    TvChartOptions {
        timeframe: TvTimeFrame::from_name(timeframe).unwrap_or(TvTimeFrame::Daily),
        range: limit,
        ..TvChartOptions::default()
    }
}

pub fn screener_request(
    market: String,
    columns: Vec<String>,
    sort_by: String,
    sort_order: String,
    limit: usize,
) -> TvScreenerRequest {
    let columns = if columns.is_empty() {
        vec![
            "name".into(),
            "close".into(),
            "change".into(),
            "volume".into(),
            "Recommend.All".into(),
        ]
    } else {
        columns
    };
    TvScreenerRequest {
        market,
        columns,
        filters: Vec::new(),
        sort_by,
        sort_order,
        range_start: 0,
        range_end: limit,
        preset: "all_stocks".into(),
    }
}
