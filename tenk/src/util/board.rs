use serde::Deserialize;

pub fn normalize_board_name(name: &str) -> String {
    let mut normalized = name.trim().to_string();
    for suffix in ["板块", "概念", "指数", "Ⅲ", "III", "行业"] {
        if let Some(stripped) = normalized.strip_suffix(suffix) {
            normalized = stripped.trim().to_string();
        }
    }
    normalized
}

pub fn parse_ths_industry_board_links(html: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let marker = "/thshy/detail/code/";
    let mut search_from = 0;
    while let Some(rel) = html[search_from..].find(marker) {
        let pos = search_from + rel + marker.len();
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
        let name = extract_link_text(&html[pos..], code_end).unwrap_or_default();
        if !name.is_empty() && !results.iter().any(|(c, _)| c == &code) {
            results.push((code, name));
        }
        search_from = pos + code_end;
    }
    results
}

pub fn parse_ths_concept_board_section(html: &str) -> Vec<(String, String, f64, f64)> {
    let Some(raw) = extract_hidden_json(html, "gnSection") else {
        return Vec::new();
    };
    #[derive(Deserialize)]
    struct SectionEntry {
        platecode: String,
        platename: String,
        #[serde(rename = "199112", default)]
        change_pct: f64,
        #[serde(default)]
        zjjlr: f64,
    }
    let parsed: Result<serde_json::Map<String, serde_json::Value>, _> =
        serde_json::from_str(&raw);
    let Ok(map) = parsed else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for value in map.values() {
        if let Ok(entry) = serde_json::from_value::<SectionEntry>(value.clone()) {
            if entry.platecode.is_empty() || entry.platename.is_empty() {
                continue;
            }
            results.push((
                entry.platecode,
                entry.platename,
                entry.change_pct,
                entry.zjjlr,
            ));
        }
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

pub fn extract_ths_news_content(html: &str) -> Option<(String, String)> {
    let marker = "class=\"news-content-parsed\"";
    let start = html.find(marker)?;
    let fragment = &html[start..];
    let open = fragment.find('>')? + 1;
    let close = fragment[open..].find("</div>")? + open;
    let body_html = fragment[open..close].trim().to_string();
    if body_html.is_empty() {
        return None;
    }
    let body_text = html2text::from_read(body_html.as_bytes(), usize::MAX)
        .unwrap_or_default()
        .trim()
        .to_string();
    Some((body_html, body_text))
}

pub fn extract_ths_news_title(html: &str) -> Option<String> {
    if let Some(start) = html.find("<h1") {
        let fragment = &html[start..];
        if let Some(open) = fragment.find('>') {
            let inner = &fragment[open + 1..];
            if let Some(close) = inner.find("</h1>") {
                let title = inner[..close]
                    .replace("<br/>", "")
                    .replace("<br>", "");
                let title = strip_html_tags(&title);
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
    }
    if let Some(start) = html.find("<title>") {
        let inner = &html[start + 7..];
        if let Some(end) = inner.find("</title>") {
            let title = inner[..end]
                .split('_')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }
    None
}

fn extract_hidden_json(html: &str, element_id: &str) -> Option<String> {
    let pattern = format!(r#"id="{element_id}" value='"#);
    let start = html.find(&pattern)? + pattern.len();
    let rest = &html[start..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

fn extract_link_text(html: &str, after_code: usize) -> Option<String> {
    let fragment = &html[after_code..];
    let open = fragment.find('>')? + 1;
    let close = fragment[open..].find('<')? + open;
    let text = fragment[open..close].trim();
    if text.is_empty() {
        return None;
    }
    Some(text.to_string())
}

fn strip_html_tags(input: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_board_name() {
        assert_eq!(normalize_board_name("半导体板块"), "半导体");
        assert_eq!(normalize_board_name("移动支付概念"), "移动支付");
    }

    #[test]
    fn test_parse_ths_industry_board_links() {
        let html = r#"<a href="http://q.10jqka.com.cn/thshy/detail/code/881121/" target="_blank">半导体</a>"#;
        let boards = parse_ths_industry_board_links(html);
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].0, "881121");
        assert_eq!(boards[0].1, "半导体");
    }

    #[test]
    fn test_parse_ths_concept_board_section() {
        let html = r#"<input type="hidden" id="gnSection" value='{"2":{"platecode":"885333","platename":"移动支付","cid":"300188","199112":-3.67,"zjjlr":-26.92,"zfl":12}}'/>"#;
        let boards = parse_ths_concept_board_section(html);
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].0, "885333");
        assert_eq!(boards[0].1, "移动支付");
        assert!((boards[0].2 + 3.67).abs() < 0.01);
    }

    #[test]
    fn test_extract_ths_news_content() {
        let html = r#"<div class="news-content-parsed"><p>测试正文</p></div>"#;
        let (body_html, body_text) = extract_ths_news_content(html).unwrap();
        assert!(body_html.contains("测试正文"));
        assert!(body_text.contains("测试正文"));
    }
}
