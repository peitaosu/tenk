pub fn extract_balanced_div(html: &str, marker: &str) -> Option<String> {
    let marker_pos = html.find(marker)?;
    let div_start = html[..marker_pos].rfind("<div")?;
    let content_start = html[div_start..].find('>')? + div_start + 1;
    let mut depth = 1usize;
    let mut cursor = content_start;
    const OPEN: &str = "<div";
    const CLOSE: &str = "</div>";
    while depth > 0 && cursor < html.len() {
        let next_open = html[cursor..].find(OPEN).map(|index| cursor + index);
        let next_close = html[cursor..].find(CLOSE).map(|index| cursor + index);
        match (next_open, next_close) {
            (_, None) => return None,
            (None, Some(close)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(html[content_start..close].to_string());
                }
                cursor = close + CLOSE.len();
            }
            (Some(open), Some(close)) if open < close => {
                depth += 1;
                cursor = open + OPEN.len();
            }
            (_, Some(close)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(html[content_start..close].to_string());
                }
                cursor = close + CLOSE.len();
            }
        }
    }
    None
}

pub fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), usize::MAX)
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_balanced_div_nested() {
        let html = r#"<div class="wrap"><div id="artibody"><p>one</p><div><span>two</span></div><p>three</p></div></div>"#;
        let body = extract_balanced_div(html, r#"id="artibody""#).unwrap();
        assert!(body.contains("one"));
        assert!(body.contains("two"));
        assert!(body.contains("three"));
    }
}
