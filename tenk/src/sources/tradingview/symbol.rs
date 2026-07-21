use crate::data::Exchange;

pub fn to_tv_symbol(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.contains(':') {
        return trimmed.to_string();
    }
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 6 {
        return match Exchange::from_stock_code(&digits) {
            Exchange::SH => format!("SSE:{digits}"),
            Exchange::SZ => format!("SZSE:{digits}"),
            Exchange::BJ => format!("BSE:{digits}"),
            Exchange::HK | Exchange::US | Exchange::Unknown => digits,
        };
    }
    if trimmed.len() <= 5 && trimmed.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return format!("NASDAQ:{}", trimmed.to_uppercase());
    }
    trimmed.to_uppercase()
}

pub fn resolve_study_id(id: &str) -> &str {
    match id {
        "STD;RSI" => "RSI@tv-basicstudies-241",
        _ => id,
    }
}

pub fn normalize_strategy_id(id: &str) -> String {
    if id.contains("Strategy") {
        id.to_string()
    } else {
        format!("{id}%1Strategy")
    }
}

pub fn auth_cookie(session: &str, signature: &str) -> String {
    if session.is_empty() {
        String::new()
    } else if signature.is_empty() {
        format!("sessionid={session}")
    } else {
        format!("sessionid={session};sessionid_sign={signature}")
    }
}

pub fn scanner_market(symbol: &str) -> &'static str {
    let tv_symbol = to_tv_symbol(symbol);
    if tv_symbol.starts_with("SSE:")
        || tv_symbol.starts_with("SZSE:")
        || tv_symbol.starts_with("BSE:")
    {
        "china"
    } else {
        "america"
    }
}

pub fn gen_session_id(prefix: &str) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    let suffix: String = (0..12)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect();
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_strategy_id() {
        assert_eq!(normalize_strategy_id("STD;RSI"), "STD;RSI%1Strategy");
        assert_eq!(
            normalize_strategy_id("STD;RSI%1Strategy"),
            "STD;RSI%1Strategy"
        );
    }

    #[test]
    fn test_resolve_study_id() {
        assert_eq!(resolve_study_id("STD;RSI"), "RSI@tv-basicstudies-241");
        assert_eq!(resolve_study_id("STD;MACD"), "STD;MACD");
    }

    #[test]
    fn test_to_tv_symbol() {
        assert_eq!(to_tv_symbol("600519"), "SSE:600519");
        assert_eq!(to_tv_symbol("000001"), "SZSE:000001");
        assert_eq!(to_tv_symbol("AAPL"), "NASDAQ:AAPL");
        assert_eq!(to_tv_symbol("BINANCE:BTCUSDT"), "BINANCE:BTCUSDT");
    }

    #[test]
    fn test_scanner_market() {
        assert_eq!(scanner_market("600519"), "china");
        assert_eq!(scanner_market("AAPL"), "america");
    }
}
