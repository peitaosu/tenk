use crate::data::Exchange;

pub fn eastmoney_secid_for_index(code: &str, exchange: Exchange) -> String {
    let prefix = match exchange {
        Exchange::SH => "1",
        Exchange::SZ | Exchange::BJ => "0",
        Exchange::Unknown => "1",
    };
    format!("{prefix}.{code}")
}

pub fn eastmoney_secid_for_board(board_code: &str) -> String {
    format!("90.{board_code}")
}

pub fn eastmoney_secid_for_hk(code: &str) -> String {
    format!("116.{code}")
}

pub fn eastmoney_secid_for_us(symbol: &str) -> String {
    format!("105.{symbol}")
}

pub fn sina_hq_symbol(code: &str, exchange: Exchange) -> String {
    format!("{}{code}", exchange.market_prefix())
}

pub fn sina_index_hq_symbol(code: &str, exchange: Exchange) -> String {
    format!("s_{}{code}", exchange.market_prefix())
}

pub fn is_hk_code(code: &str) -> bool {
    code.len() == 5 && code.chars().all(|c| c.is_ascii_digit())
}

pub fn is_us_symbol(code: &str) -> bool {
    code.chars().all(|c| c.is_ascii_alphabetic()) && !code.starts_with("BK")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secid_helpers() {
        assert_eq!(eastmoney_secid_for_hk("00700"), "116.00700");
        assert_eq!(eastmoney_secid_for_us("AAPL"), "105.AAPL");
        assert_eq!(eastmoney_secid_for_board("BK1051"), "90.BK1051");
        assert_eq!(sina_index_hq_symbol("000001", Exchange::SH), "s_sh000001");
    }

    #[test]
    fn test_symbol_kind_detection() {
        assert!(is_hk_code("00700"));
        assert!(is_us_symbol("AAPL"));
        assert!(!is_us_symbol("600519"));
    }
}
