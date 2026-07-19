/// Extracts JSON object from a JSONP response body.
pub fn parse_jsonp(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if start < end {
        Some(&text[start..=end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jsonp() {
        let text = r#"callback({"data": "test"})"#;
        assert_eq!(parse_jsonp(text), Some(r#"{"data": "test"}"#));
    }

    #[test]
    fn test_parse_jsonp_nested_braces() {
        let text = r#"jQuery123({"outer":{"inner":1},"ok":true});"#;
        assert_eq!(
            parse_jsonp(text),
            Some(r#"{"outer":{"inner":1},"ok":true}"#)
        );
    }

    #[test]
    fn test_parse_jsonp_no_json() {
        assert_eq!(parse_jsonp("no json here"), None);
        assert_eq!(parse_jsonp("{"), None);
        assert_eq!(parse_jsonp("}{"), None);
    }
}
