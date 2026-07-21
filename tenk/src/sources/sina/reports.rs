use async_trait::async_trait;
use tracing::debug;

use crate::data::{Exchange, ResearchReportData};
use crate::error::{DataError, DataResult};
use crate::traits::ResearchReportSource;
use crate::util::{decode_gb18030, parse_trade_date};

use super::SinaSource;

impl SinaSource {
    fn report_list_url(stock_code: Option<&str>) -> String {
        match stock_code {
            Some(code) => format!(
                "https://stock.finance.sina.com.cn/stock/go.php/vReport_List/kind/search/index.phtml?t1=all&symbol={}{}",
                Self::market_prefix(code),
                code
            ),
            None => "https://stock.finance.sina.com.cn/stock/go.php/vReport_List/kind/lastest/index.phtml"
                .to_string(),
        }
    }

    fn parse_report_rows(html: &str) -> Vec<ResearchReportData> {
        let marker = "/stock/go.php/vReport_Show/kind/";
        let mut reports = Vec::new();
        let mut offset = 0;
        while let Some(rel) = html[offset..].find(marker) {
            let start = offset + rel;
            let slice = &html[start..];
            let Some(report_id) = extract_between(slice, "rptid/", "/index.phtml") else {
                offset = start + marker.len();
                continue;
            };
            let Some(title_raw) = extract_between(slice, "/index.phtml\">", "</a>") else {
                offset = start + marker.len();
                continue;
            };
            let row_end = slice.find("</tr>").unwrap_or(slice.len());
            let row = &slice[..row_end];
            let cells = extract_cells_after_title(row);
            let title = strip_html(title_raw);
            let publish_date = cells
                .first()
                .map(|value| parse_trade_date(value.trim()))
                .unwrap_or_else(|| parse_trade_date(""));
            let institution = cells.get(1).cloned().unwrap_or_default();
            let analysts = cells.get(2).cloned().unwrap_or_default();
            let (stock_code, stock_name) = parse_stock_from_title(&title);
            reports.push(ResearchReportData {
                report_id: report_id.to_string(),
                stock_code,
                stock_name,
                title,
                institution,
                analysts,
                rating: None,
                publish_date,
            });
            offset = start + marker.len();
        }
        reports
    }
}

fn extract_between<'a>(input: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = input.find(start_marker)? + start_marker.len();
    let end = input[start..].find(end_marker)? + start;
    Some(&input[start..end])
}

fn extract_cells_after_title(row: &str) -> Vec<String> {
    row.split("</td>")
        .skip(2)
        .take(3)
        .map(strip_html)
        .collect()
}

fn parse_stock_from_title(title: &str) -> (String, String) {
    let Some(open) = title.find('(') else {
        return (String::new(), title.to_string());
    };
    let Some(close) = title[open..].find(')') else {
        return (String::new(), title.to_string());
    };
    let stock_code = title[open + 1..open + close].trim().to_string();
    let stock_name = title[..open].trim().to_string();
    (stock_code, stock_name)
}

fn strip_html(input: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[async_trait]
impl ResearchReportSource for SinaSource {
    async fn get_research_reports(
        &self,
        stock_code: Option<&str>,
        page: u32,
        limit: Option<usize>,
    ) -> DataResult<Vec<ResearchReportData>> {
        if let Some(code) = stock_code {
            if Exchange::from_stock_code(code) == Exchange::Unknown {
                return Err(DataError::custom("invalid stock code"));
            }
        }

        let url = Self::report_list_url(stock_code);
        debug!("Fetching Sina research reports: {}", url);
        let response = self.finance_request().get(&url).await?;
        let bytes = response.bytes().await.map_err(DataError::Network)?;
        let html = decode_gb18030(&bytes);
        let mut reports = Self::parse_report_rows(&html);
        let page_size = limit.unwrap_or(50);
        let start = (page.max(1) as usize - 1).saturating_mul(page_size);
        if start > 0 {
            if start >= reports.len() {
                reports.clear();
            } else {
                reports = reports.into_iter().skip(start).collect();
            }
        }
        reports.truncate(page_size);
        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_report_rows() {
        let html = r#"<tr><td class="tal f14"><a href="/stock/go.php/vReport_Show/kind/search/rptid/837811291904/index.phtml">贵州茅台(600519)点评</a></td><td>公司</td><td>2026-07-19</td><td><a><div class="fname05"><span>申万宏源</span></div></a></td><td><div class="fname"><span>张三/李四</span></div></td></tr>"#;
        let rows = SinaSource::parse_report_rows(html);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].report_id, "837811291904");
        assert_eq!(rows[0].stock_code, "600519");
        assert_eq!(rows[0].institution, "申万宏源");
    }
}
