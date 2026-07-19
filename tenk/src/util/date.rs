use chrono::NaiveDate;

const EPOCH_DATE: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(d) => d,
    None => panic!("invalid epoch date"),
};

pub fn parse_trade_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or(EPOCH_DATE)
}

pub fn normalize_date_bound(value: Option<&str>, default: &str) -> String {
    match value {
        None => default.to_string(),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                format!("{}-{}-{}", &trimmed[0..4], &trimmed[4..6], &trimmed[6..8])
            } else {
                trimmed.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_date_bound() {
        assert_eq!(
            normalize_date_bound(Some("20250115"), "1990-01-01"),
            "2025-01-15"
        );
        assert_eq!(
            normalize_date_bound(Some("2025-01-15"), "1990-01-01"),
            "2025-01-15"
        );
    }

    #[test]
    fn test_parse_trade_date() {
        assert_eq!(
            parse_trade_date("2025-01-15"),
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
        assert_eq!(parse_trade_date("bad"), EPOCH_DATE);
        assert_eq!(parse_trade_date(""), EPOCH_DATE);
    }
}
