use chrono::{NaiveDateTime, TimeZone, Utc};

use crate::data::{OrderBookData, TickData};

pub fn parse_order_book_from_fields(
    stock_code: &str,
    short_name: &str,
    buy_price: [Option<i64>; 5],
    buy_volume: [Option<i64>; 5],
    sell_price: [Option<i64>; 5],
    sell_volume: [Option<i64>; 5],
) -> OrderBookData {
    let scale_price = |v: Option<i64>| v.map(|p| p as f64 / 100.0).unwrap_or(0.0);
    let scale_vol = |v: Option<i64>| v.map(|n| n as u64).unwrap_or(0);
    OrderBookData {
        stock_code: stock_code.to_string(),
        short_name: short_name.to_string(),
        buy_prices: [
            scale_price(buy_price[0]),
            scale_price(buy_price[1]),
            scale_price(buy_price[2]),
            scale_price(buy_price[3]),
            scale_price(buy_price[4]),
        ],
        buy_volumes: [
            scale_vol(buy_volume[0]),
            scale_vol(buy_volume[1]),
            scale_vol(buy_volume[2]),
            scale_vol(buy_volume[3]),
            scale_vol(buy_volume[4]),
        ],
        sell_prices: [
            scale_price(sell_price[0]),
            scale_price(sell_price[1]),
            scale_price(sell_price[2]),
            scale_price(sell_price[3]),
            scale_price(sell_price[4]),
        ],
        sell_volumes: [
            scale_vol(sell_volume[0]),
            scale_vol(sell_volume[1]),
            scale_vol(sell_volume[2]),
            scale_vol(sell_volume[3]),
            scale_vol(sell_volume[4]),
        ],
    }
}

pub fn parse_tick_details(stock_code: &str, details: &[String]) -> Vec<TickData> {
    let today = Utc::now().date_naive();
    details
        .iter()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 {
                return None;
            }
            let time_str = parts[0];
            let price: f64 = parts[1].parse().ok()?;
            let volume: u64 = parts[2].parse().ok()?;
            let direction = match parts.get(3).copied().unwrap_or("0") {
                "1" => 'B',
                "2" => 'S',
                _ => '-',
            };
            let trade_time = NaiveDateTime::parse_from_str(
                &format!("{today} {time_str}"),
                "%Y-%m-%d %H:%M:%S",
            )
            .map(|dt| Utc.from_utc_datetime(&dt))
            .unwrap_or_else(|_| Utc.from_utc_datetime(&today.and_hms_opt(0, 0, 0).unwrap()));
            Some(TickData {
                stock_code: stock_code.to_string(),
                trade_time,
                price,
                volume,
                direction,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_order_book_from_fields() {
        let book = parse_order_book_from_fields(
            "600519",
            "贵州茅台",
            [Some(125299), None, None, None, None],
            [Some(100), None, None, None, None],
            [Some(125300), None, None, None, None],
            [Some(200), None, None, None, None],
        );
        assert_eq!(book.buy_prices[0], 1252.99);
        assert_eq!(book.sell_prices[0], 1253.0);
    }

    #[test]
    fn test_parse_tick_details() {
        let ticks = parse_tick_details(
            "600519",
            &["15:05:57,1253.00,100,2,1".to_string()],
        );
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].direction, 'S');
    }
}
