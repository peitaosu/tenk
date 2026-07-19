//! News command handlers.

use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use comfy_table::Cell;
use rust_i18n::t;
use tenk::{DataClient, NewsArticle, NewsCategory, format_related_stocks_display};

use crate::NewsAction;
use crate::output::{OutputConfig, OutputFormat, TableRow, print_output};

/// Handles news commands.
pub async fn handle(action: NewsAction, client: &DataClient, config: &OutputConfig) -> Result<()> {
    match action {
        NewsAction::List {
            category,
            page,
            limit,
        } => {
            let cat = NewsCategory::from_name(&category);
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
                eprintln!("{}", t!("messages.no_news_found").yellow());
            } else {
                eprintln!(
                    "{}",
                    t!("messages.found_news", count = data.len(), keyword = keyword)
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
                        t!("labels.source").bright_black(),
                        content.source.yellow(),
                        t!("labels.time").bright_black(),
                        local_time.bright_black()
                    );
                    if let Some(ref author) = content.author {
                        println!("{} {}", t!("labels.author").bright_black(), author);
                    }
                    if !content.related_stocks.is_empty() {
                        let (stocks, sectors) =
                            format_related_stocks_display(&content.related_stocks);
                        if !stocks.is_empty() {
                            println!(
                                "{} {}",
                                t!("labels.stocks").bright_black(),
                                stocks.join("  ").yellow()
                            );
                        }
                        if !sectors.is_empty() {
                            println!(
                                "{} {}",
                                t!("labels.sectors").bright_black(),
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

impl TableRow for NewsArticle {
    fn headers() -> Vec<Cell> {
        vec![
            Cell::new(t!("headers.id")),
            Cell::new(t!("headers.time")),
            Cell::new(t!("headers.title")),
            Cell::new(t!("headers.source")),
        ]
    }

    fn row(&self) -> Vec<Cell> {
        let local_time = Local
            .from_utc_datetime(&self.publish_time.naive_utc())
            .format("%m-%d %H:%M")
            .to_string();

        let title = self.title.clone();

        vec![
            Cell::new(&self.id),
            Cell::new(local_time),
            Cell::new(title),
            Cell::new(&self.source),
        ]
    }
}
