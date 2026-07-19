use crate::data::KLineType;
use crate::error::{DataError, DataResult};

pub fn kline_period_code(k_type: KLineType) -> DataResult<&'static str> {
    match k_type {
        KLineType::Daily => Ok("01"),
        KLineType::Weekly => Ok("11"),
        KLineType::Monthly => Ok("21"),
        KLineType::Min30 => Ok("30"),
        KLineType::Min60 => Ok("60"),
        other => Err(DataError::not_supported(format!(
            "THS does not support kline type {other:?}"
        ))),
    }
}

pub fn is_board_antibot_page(html: &str) -> bool {
    html.contains("chameleon") && !html.contains("m-table")
}

pub fn parse_board_html(html: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = html[search_from..].find("stockpage.10jqka.com.cn/") {
        let pos = search_from + rel + "stockpage.10jqka.com.cn/".len();
        let rest = &html[pos..];
        let code_end = rest
            .find('/')
            .or_else(|| rest.find('"'))
            .unwrap_or(rest.len());
        let code = rest[..code_end].trim().to_string();
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            search_from = pos;
            continue;
        }
        let name = extract_anchor_text_after(&html[pos..], code_end).unwrap_or_default();
        if !results.iter().any(|(c, _)| c == &code) {
            results.push((code, name));
        }
        search_from = pos + code_end;
    }
    results
}

fn extract_anchor_text_after(html: &str, after_code: usize) -> Option<String> {
    let fragment = &html[after_code..];
    let open = fragment.find('>')? + 1;
    let close = fragment[open..].find('<')? + open;
    let text = fragment[open..close].trim();
    if text.is_empty() || text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_period_code() {
        assert_eq!(kline_period_code(KLineType::Daily).unwrap(), "01");
        assert_eq!(kline_period_code(KLineType::Min30).unwrap(), "30");
        assert!(kline_period_code(KLineType::Min5).is_err());
    }

    #[test]
    fn test_parse_board_html() {
        let html = r#"<table class="m-table"><tr><td><a href="http://stockpage.10jqka.com.cn/600519/">600519</a></td><td><a>贵州茅台</a></td></tr></table>"#;
        let codes = parse_board_html(html);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].0, "600519");
    }

    #[test]
    fn test_is_board_antibot_page() {
        assert!(is_board_antibot_page("<script>chameleon</script>"));
        assert!(!is_board_antibot_page("<table class=\"m-table\">chameleon</table>"));
    }
}
