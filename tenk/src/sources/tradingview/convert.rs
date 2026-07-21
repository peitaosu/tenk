use chrono::{NaiveDate, TimeZone};

use super::protocol::normalize_unix_timestamp;

use crate::data::{
    CurrentMarketData, KLineType, MinuteData, TvChartBar, TvChartOptions, TvQuote,
    TvTimeFrame,
};

use super::symbol::to_tv_symbol;

pub fn kline_type_to_timeframe(k_type: KLineType) -> TvTimeFrame {
    match k_type {
        KLineType::Daily => TvTimeFrame::Daily,
        KLineType::Weekly => TvTimeFrame::Weekly,
        KLineType::Monthly | KLineType::Quarterly => TvTimeFrame::Monthly,
        KLineType::Min5 => TvTimeFrame::Min5,
        KLineType::Min15 => TvTimeFrame::Min15,
        KLineType::Min30 => TvTimeFrame::Min30,
        KLineType::Min60 => TvTimeFrame::Min60,
    }
}

pub fn chart_options_for_kline(
    k_type: KLineType,
    start_date: Option<&str>,
    end_date: Option<&str>,
    limit: usize,
) -> TvChartOptions {
    let mut options = TvChartOptions {
        timeframe: kline_type_to_timeframe(k_type),
        range: limit,
        ..TvChartOptions::default()
    };
    if let Some(end) = end_date.or(start_date) {
        if let Ok(date) = NaiveDate::parse_from_str(end, "%Y-%m-%d") {
            if let Some(dt) = date.and_hms_opt(23, 59, 59) {
                options.to = Some(dt.and_utc().timestamp());
            }
        } else if let Ok(date) = NaiveDate::parse_from_str(end, "%Y%m%d") {
            if let Some(dt) = date.and_hms_opt(23, 59, 59) {
                options.to = Some(dt.and_utc().timestamp());
            }
        }
    }
    options
}

pub fn tv_quote_to_current(quote: &TvQuote) -> CurrentMarketData {
    let code = quote.symbol.rsplit(':').next().unwrap_or(&quote.symbol).to_string();
    CurrentMarketData {
        stock_code: code,
        short_name: quote
            .description
            .clone()
            .unwrap_or_else(|| quote.symbol.clone()),
        price: quote.last_price.unwrap_or(quote.prev_close.unwrap_or(0.0)),
        change: quote.change.unwrap_or(0.0),
        change_pct: quote.change_percent.unwrap_or(0.0),
        volume: quote.volume.unwrap_or(0.0) as u64,
        amount: 0.0,
        open: quote.open,
        high: quote.high,
        low: quote.low,
        pre_close: quote.prev_close,
    }
}

pub fn tv_bar_to_minute(bar: &TvChartBar, symbol: &str) -> MinuteData {
    let code = symbol.rsplit(':').next().unwrap_or(symbol).to_string();
    let trade_time = chrono::Utc
        .timestamp_opt(normalize_unix_timestamp(bar.time), 0)
        .single()
        .unwrap_or_else(chrono::Utc::now);
    let change = bar.close - bar.open;
    let change_pct = if bar.open > 0.0 {
        change / bar.open * 100.0
    } else {
        0.0
    };
    MinuteData {
        stock_code: code,
        trade_time,
        price: bar.close,
        change,
        change_pct,
        volume: bar.volume as u64,
        avg_price: (bar.high + bar.low) / 2.0,
        amount: 0.0,
    }
}

pub fn to_hk_tv_symbol(code: &str) -> String {
    if code.contains(':') {
        return code.to_string();
    }
    let trimmed = code.trim().trim_start_matches('0');
    let digits = if trimmed.is_empty() { code.trim() } else { trimmed };
    format!("HKEX:{digits}")
}

pub fn to_us_tv_symbol(symbol: &str) -> String {
    if symbol.contains(':') {
        return symbol.to_string();
    }
    format!("NASDAQ:{symbol}")
}

pub fn normalize_market_symbol(symbol: &str) -> String {
    to_tv_symbol(symbol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::TvChartBar;

    #[test]
    fn test_to_tv_symbol() {
        assert_eq!(to_tv_symbol("600519"), "SSE:600519");
        assert_eq!(to_tv_symbol("000001"), "SZSE:000001");
        assert_eq!(to_tv_symbol("BINANCE:BTCUSDT"), "BINANCE:BTCUSDT");
    }

    #[test]
    fn test_tv_bar_to_minute_timestamp() {
        let bar = TvChartBar {
            time: 1_704_067_200,
            open: 100.0,
            high: 110.0,
            low: 90.0,
            close: 105.0,
            volume: 1000.0,
        };
        let minute = tv_bar_to_minute(&bar, "SSE:600519");
        assert_eq!(minute.stock_code, "600519");
        assert_eq!(minute.trade_time.timestamp(), 1_704_067_200);
    }
}
