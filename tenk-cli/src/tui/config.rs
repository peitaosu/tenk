use std::fs;
use std::path::PathBuf;

use tenk::{Exchange, StockCode};

const DEFAULT_SYMBOLS: &[(&str, Exchange, &str)] = &[
    ("600519", Exchange::SH, "贵州茅台"),
    ("000001", Exchange::SZ, "平安银行"),
    ("510300", Exchange::SH, "沪深300ETF"),
];

pub fn watchlist_path() -> PathBuf {
    home_dir().join(".config/tenk/watchlist.txt")
}

pub fn load_watchlist() -> Vec<StockCode> {
    let path = watchlist_path();
    if let Ok(text) = fs::read_to_string(&path) {
        let symbols: Vec<StockCode> = text
            .lines()
            .filter_map(parse_watchlist_line)
            .collect();
        if !symbols.is_empty() {
            return symbols;
        }
    }
    DEFAULT_SYMBOLS
        .iter()
        .map(|(code, exchange, name)| StockCode::new(code.to_string(), (*name).to_string(), *exchange))
        .collect()
}

pub fn save_watchlist(symbols: &[StockCode]) -> std::io::Result<()> {
    let path = watchlist_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body: Vec<String> = symbols.iter().map(format_watchlist_line).collect();
    fs::write(path, body.join("\n") + "\n")
}

fn parse_watchlist_line(line: &str) -> Option<StockCode> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    if let Some((code, exchange_text)) = line.split_once(':') {
        let code = code.trim();
        let exchange = exchange_text.trim().parse::<Exchange>().ok()?;
        if code.is_empty() || exchange == Exchange::Unknown {
            return None;
        }
        return Some(StockCode::new(code.to_string(), String::new(), exchange));
    }
    let parts: Vec<&str> = line
        .split('\t')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    match parts.as_slice() {
        [code, exchange_text] => {
            let exchange = exchange_text.parse::<Exchange>().ok()?;
            if code.is_empty() || exchange == Exchange::Unknown {
                return None;
            }
            Some(StockCode::new(code.to_string(), String::new(), exchange))
        }
        [code, exchange_text, name] => {
            let exchange = exchange_text.parse::<Exchange>().ok()?;
            if code.is_empty() || exchange == Exchange::Unknown {
                return None;
            }
            Some(StockCode::new(code.to_string(), name.to_string(), exchange))
        }
        _ => None,
    }
}

fn format_watchlist_line(symbol: &StockCode) -> String {
    if symbol.short_name.is_empty() {
        format!("{}\t{}", symbol.stock_code, symbol.exchange)
    } else {
        format!(
            "{}\t{}\t{}",
            symbol.stock_code, symbol.exchange, symbol.short_name
        )
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
