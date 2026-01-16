//! News command handlers.

use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use comfy_table::Cell;
use tenk::{DataClient, NewsArticle, NewsCategory};

use crate::output::{print_output, OutputConfig, OutputFormat, TableRow};
use crate::NewsAction;

/// Handles news commands.
pub async fn handle(action: NewsAction, client: &DataClient, config: &OutputConfig) -> Result<()> {
    match action {
        NewsAction::List {
            category,
            page,
            limit,
        } => {
            let cat = parse_category(&category);
            let data = client.get_news(cat, page, limit).await?;
            print_output(&data, config);
        }
        NewsAction::Search {
            keyword,
            page,
            limit,
        } => {
            let data = client.search_news(&keyword, page, limit).await?;
            if data.is_empty() {
                eprintln!("{}", "No news found.".yellow());
            } else {
                eprintln!(
                    "{}",
                    format!("Found {} news for '{}':", data.len(), keyword)
                        .cyan()
                        .bold()
                );
                print_output(&data, config);
            }
        }
        NewsAction::Read { id } => {
            let content = client.get_news_content(&id).await?;

            match config.format {
                OutputFormat::JSON => {
                    let json = serde_json::to_string_pretty(&content)?;
                    if let Some(ref file) = config.file {
                        std::fs::write(file, &json)?;
                    } else {
                        println!("{}", json);
                    }
                }
                _ => {
                    let local_time = Local
                        .from_utc_datetime(&content.publish_time.naive_utc())
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();

                    println!("{}", "─".repeat(60).bright_black());
                    println!("{}", content.title.bold().cyan());
                    println!("{}", "─".repeat(60).bright_black());
                    println!(
                        "{} {} | {} {}",
                        "Source:".bright_black(),
                        content.source.yellow(),
                        "Time:".bright_black(),
                        local_time.bright_black()
                    );
                    if let Some(ref author) = content.author {
                        println!("{} {}", "Author:".bright_black(), author);
                    }
                    if !content.related_stocks.is_empty() {
                        let (stocks, sectors) = format_related_stocks(&content.related_stocks);
                        if !stocks.is_empty() {
                            println!(
                                "{} {}",
                                "Stocks:".bright_black(),
                                stocks.join("  ").yellow()
                            );
                        }
                        if !sectors.is_empty() {
                            println!(
                                "{} {}",
                                "Sectors:".bright_black(),
                                sectors.join("  ").magenta()
                            );
                        }
                    }
                    println!("{}", "─".repeat(60).bright_black());
                    println!();
                    println!("{}", content.body_text);
                    println!();
                    println!("{}", "─".repeat(60).bright_black());
                }
            }
        }
    }
    Ok(())
}

fn format_related_stocks(codes: &[String]) -> (Vec<String>, Vec<String>) {
    let mut stocks = Vec::new();
    let mut sectors = Vec::new();

    for code in codes {
        if let Some((market, symbol)) = code.split_once('.') {
            let formatted = match market {
                "0" => format!("{}.SZ", symbol),
                "1" => format!("{}.SH", symbol),
                "90" => {
                    sectors.push(symbol.to_string());
                    continue;
                }
                "105" => format!("{} (NASDAQ)", symbol),
                "106" => format!("{} (NYSE)", symbol),
                "116" => format!("{}.HK", symbol),
                "118" => format!("{} (KR)", symbol),
                _ => code.clone(),
            };
            stocks.push(formatted);
        } else {
            stocks.push(code.clone());
        }
    }

    (stocks, sectors)
}

fn parse_category(s: &str) -> NewsCategory {
    match s.to_lowercase().as_str() {
        "finance" | "102" => NewsCategory::Finance,
        "company" | "103" => NewsCategory::Company,
        "stock" | "104" => NewsCategory::Stock,
        "us" | "usmarket" | "105" => NewsCategory::USMarket,
        "global" | "111" => NewsCategory::Global,
        "domestic" | "106" => NewsCategory::Domestic,
        "industry" | "115" => NewsCategory::Industry,
        _ => NewsCategory::Finance,
    }
}

impl TableRow for NewsArticle {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new("Time"),
            Cell::new("Title"),
            Cell::new("Source"),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        let local_time = Local
            .from_utc_datetime(&self.publish_time.naive_utc())
            .format("%m-%d %H:%M")
            .to_string();

        
        let title = self.title.clone();

        vec![
            Cell::new(local_time),
            Cell::new(title),
            Cell::new(&self.source),
        ]
    }
}
