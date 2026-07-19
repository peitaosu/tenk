//! Related stock code from EastMoney encoding.

use serde::Serialize;

/// Parsed related stock reference.
#[derive(Debug, Clone, Serialize)]
pub struct RelatedStock {
    pub symbol: String,
    pub market: String,
    pub formatted: String,
}

/// Splits EastMoney related codes into stocks and sector names.
pub fn format_related_stocks(codes: &[String]) -> (Vec<RelatedStock>, Vec<String>) {
    let mut stocks = Vec::new();
    let mut sectors = Vec::new();

    for code in codes {
        let Some((market, symbol)) = code.split_once('.') else {
            continue;
        };
        match market {
            "0" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "SZ".to_string(),
                formatted: format!("{symbol}.SZ"),
            }),
            "1" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "SH".to_string(),
                formatted: format!("{symbol}.SH"),
            }),
            "90" => sectors.push(symbol.to_string()),
            "105" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "NASDAQ".to_string(),
                formatted: format!("{symbol} (NASDAQ)"),
            }),
            "106" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "NYSE".to_string(),
                formatted: format!("{symbol} (NYSE)"),
            }),
            "116" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "HK".to_string(),
                formatted: format!("{symbol}.HK"),
            }),
            "118" => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: "KR".to_string(),
                formatted: format!("{symbol} (KR)"),
            }),
            _ => stocks.push(RelatedStock {
                symbol: symbol.to_string(),
                market: market.to_string(),
                formatted: code.clone(),
            }),
        }
    }

    (stocks, sectors)
}

/// Formats related stocks as display strings for CLI output.
pub fn format_related_stocks_display(codes: &[String]) -> (Vec<String>, Vec<String>) {
    let (stocks, sectors) = format_related_stocks(codes);
    (
        stocks.into_iter().map(|s| s.formatted).collect(),
        sectors,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_related_stocks() {
        let codes = vec!["1.600519".to_string(), "90.白酒".to_string()];
        let (stocks, sectors) = format_related_stocks(&codes);
        assert_eq!(stocks.len(), 1);
        assert_eq!(stocks[0].formatted, "600519.SH");
        assert_eq!(sectors, vec!["白酒"]);
    }

    #[test]
    fn test_format_related_stocks_sz_and_hk() {
        let codes = vec!["0.300059".to_string(), "116.00700".to_string()];
        let (stocks, sectors) = format_related_stocks(&codes);
        assert_eq!(stocks.len(), 2);
        assert_eq!(stocks[0].formatted, "300059.SZ");
        assert_eq!(stocks[1].formatted, "00700.HK");
        assert!(sectors.is_empty());
    }

    #[test]
    fn test_format_related_stocks_skips_invalid() {
        let codes = vec!["invalid".to_string(), "1.600519".to_string()];
        let (stocks, _) = format_related_stocks(&codes);
        assert_eq!(stocks.len(), 1);
    }

    #[test]
    fn test_format_related_stocks_display() {
        let codes = vec!["1.600519".to_string(), "90.白酒".to_string()];
        let (stock_labels, sectors) = format_related_stocks_display(&codes);
        assert_eq!(stock_labels, vec!["600519.SH"]);
        assert_eq!(sectors, vec!["白酒"]);
    }
}
