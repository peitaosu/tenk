use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::data::{KLineType, MarketData, MinuteData, OrderBookData, TickData};
use crate::util::{normalize_date_bound, parse_trade_date};

pub fn decode_gb18030(bytes: &[u8]) -> String {
    let (decoded, _, _) = encoding_rs::GB18030.decode(bytes);
    decoded.into_owned()
}

pub fn kline_scale(k_type: KLineType) -> Option<u32> {
    match k_type {
        KLineType::Min5 => Some(5),
        KLineType::Min15 => Some(15),
        KLineType::Min30 => Some(30),
        KLineType::Min60 => Some(60),
        KLineType::Daily => Some(240),
        KLineType::Weekly => Some(1200),
        KLineType::Monthly => Some(7200),
        KLineType::Quarterly => None,
    }
}

pub fn parse_order_book_from_parts(
    stock_code: &str,
    short_name: &str,
    parts: &[&str],
) -> Option<OrderBookData> {
    if parts.len() < 30 {
        return None;
    }
    let mut buy_prices = [0.0; 5];
    let mut buy_volumes = [0u64; 5];
    let mut sell_prices = [0.0; 5];
    let mut sell_volumes = [0u64; 5];
    for level in 0..5 {
        let base = 10 + level * 2;
        buy_volumes[level] = parts[base].parse().unwrap_or(0);
        buy_prices[level] = parts[base + 1].parse().unwrap_or(0.0);
        let ask_base = 20 + level * 2;
        sell_volumes[level] = parts[ask_base].parse().unwrap_or(0);
        sell_prices[level] = parts[ask_base + 1].parse().unwrap_or(0.0);
    }
    Some(OrderBookData {
        stock_code: stock_code.to_string(),
        short_name: short_name.to_string(),
        sell_prices,
        sell_volumes,
        buy_prices,
        buy_volumes,
    })
}

pub fn parse_ticks_from_trans_list(text: &str, stock_code: &str) -> Vec<TickData> {
    let today = Utc::now().date_naive();
    let mut ticks = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.contains("trade_item_list") || !line.contains("new Array") {
            continue;
        }
        let start = match line.find('(') {
            Some(i) => i + 1,
            None => continue,
        };
        let end = match line.rfind(')') {
            Some(i) => i,
            None => continue,
        };
        let inner = &line[start..end];
        let fields: Vec<&str> = inner.split(',').map(|s| s.trim().trim_matches('\'')).collect();
        if fields.len() < 4 {
            continue;
        }
        let time_str = fields[0];
        let volume: u64 = fields[1].parse().unwrap_or(0);
        let price: f64 = fields[2].parse().unwrap_or(0.0);
        let direction = match fields.get(3).copied().unwrap_or("-") {
            "UP" => 'B',
            "DOWN" => 'S',
            _ => '-',
        };
        let trade_time = NaiveDateTime::parse_from_str(
            &format!("{today} {time_str}"),
            "%Y-%m-%d %H:%M:%S",
        )
        .map(|dt| Utc.from_utc_datetime(&dt))
        .unwrap_or_else(|_| Utc.from_utc_datetime(&today.and_hms_opt(0, 0, 0).unwrap()));
        ticks.push(TickData {
            stock_code: stock_code.to_string(),
            trade_time,
            price,
            volume,
            direction,
        });
    }
    ticks
}

pub fn parse_kline_records(
    stock_code: &str,
    records: &[SinaKLineRecord],
    start_date: Option<&str>,
    end_date: Option<&str>,
    intraday: bool,
) -> Vec<MarketData> {
    let start = normalize_date_bound(start_date, "1990-01-01");
    let end = normalize_date_bound(end_date, "2099-12-31");
    let mut result = Vec::with_capacity(records.len());
    for record in records {
        let (trade_date, trade_time) = if intraday {
            let dt = NaiveDateTime::parse_from_str(&record.day, "%Y-%m-%d %H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(&record.day, "%Y-%m-%d %H:%M"));
            let dt = match dt {
                Ok(d) => d,
                Err(_) => continue,
            };
            (dt.date(), Utc.from_utc_datetime(&dt))
        } else {
            let date = parse_trade_date(&record.day[..record.day.len().min(10)]);
            (
                date,
                Utc.from_utc_datetime(&date.and_hms_opt(15, 0, 0).unwrap_or_default()),
            )
        };
        let trade_date_str = trade_date.format("%Y-%m-%d").to_string();
        if trade_date_str.as_str() < start.as_str() || trade_date_str.as_str() > end.as_str() {
            continue;
        }
        let open: f64 = record.open.parse().unwrap_or(0.0);
        let high: f64 = record.high.parse().unwrap_or(0.0);
        let low: f64 = record.low.parse().unwrap_or(0.0);
        let close: f64 = record.close.parse().unwrap_or(0.0);
        let volume: u64 = record.volume.parse().unwrap_or(0);
        let amount = 0.0;
        result.push(MarketData {
            stock_code: stock_code.to_string(),
            trade_time,
            trade_date,
            open,
            close,
            high,
            low,
            volume,
            amount,
            change: 0.0,
            change_pct: 0.0,
            turnover_ratio: 0.0,
            pre_close: open,
        });
    }
    if result.len() > 1 && !intraday {
        for i in 1..result.len() {
            let prev_close = result[i - 1].close;
            result[i].pre_close = prev_close;
            result[i].change = result[i].close - prev_close;
            result[i].change_pct = if prev_close > 0.0 {
                (result[i].close - prev_close) / prev_close * 100.0
            } else {
                0.0
            };
        }
    }
    result
}

pub fn parse_minute_records(stock_code: &str, date: NaiveDate, records: &[SinaMinuteRecord]) -> Vec<MinuteData> {
    records
        .iter()
        .filter_map(|record| {
            let time_str = format!("{date} {}", record.m);
            let trade_time = NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                .map(|dt| Utc.from_utc_datetime(&dt))
                .ok()?;
            Some(MinuteData {
                stock_code: stock_code.to_string(),
                trade_time,
                price: record.p.parse().unwrap_or(0.0),
                change: 0.0,
                change_pct: 0.0,
                volume: record.v.parse().unwrap_or(0),
                avg_price: record.avg_p.parse().unwrap_or(0.0),
                amount: 0.0,
            })
        })
        .collect()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SinaKLineRecord {
    pub day: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SinaMinuteRecord {
    pub m: String,
    pub v: String,
    pub p: String,
    #[serde(rename = "avg_p")]
    pub avg_p: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SinaMinuteResponse {
    pub result: Option<SinaMinuteResult>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SinaMinuteResult {
    pub data: Option<Vec<SinaMinuteRecord>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_scale_mapping() {
        assert_eq!(kline_scale(KLineType::Daily), Some(240));
        assert_eq!(kline_scale(KLineType::Min5), Some(5));
        assert!(kline_scale(KLineType::Quarterly).is_none());
    }

    #[test]
    fn test_parse_order_book_from_parts() {
        let parts: Vec<&str> = "n,o,pc,p,h,l,bp,sp,v,a,3600,1252.99,100,1252.97,100,1252.96,100,1252.90,100,1252.70,1953,1253.00,100,1253.32,100,1253.75,200,1254.00,100,1254.40,d,t,s"
            .split(',')
            .collect();
        let book = parse_order_book_from_parts("600519", "贵州茅台", &parts).unwrap();
        assert_eq!(book.buy_volumes[0], 3600);
        assert_eq!(book.buy_prices[0], 1252.99);
        assert_eq!(book.sell_prices[0], 1253.00);
    }

    #[test]
    fn test_parse_ticks_from_trans_list() {
        let text = r#"trade_item_list[0] = new Array('15:00:01', '55700', '1253.000', 'DOWN');"#;
        let ticks = parse_ticks_from_trans_list(text, "600519");
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].volume, 55700);
        assert_eq!(ticks[0].direction, 'S');
    }

    #[test]
    fn test_parse_kline_records_daily() {
        let records = vec![SinaKLineRecord {
            day: "2026-07-17".to_string(),
            open: "1269.01".to_string(),
            high: "1269.33".to_string(),
            low: "1238.98".to_string(),
            close: "1253.00".to_string(),
            volume: "5841730".to_string(),
        }];
        let data = parse_kline_records("600519", &records, None, None, false);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].close, 1253.0);
    }
}
