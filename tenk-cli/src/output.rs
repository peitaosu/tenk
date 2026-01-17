//! Output formatting utilities.

use clap::ValueEnum;
use colored::Colorize;
use comfy_table::{
    Cell, CellAlignment, Color, ContentArrangement, Table, presets::UTF8_FULL_CONDENSED,
};
use serde::Serialize;
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// Output format for CLI results.
#[derive(Clone, Copy, ValueEnum, Debug, Default)]
pub enum OutputFormat {
    /// JSON format
    JSON,
    /// Table format
    #[default]
    Table,
    /// CSV format
    CSV,
}

/// Output configuration.
#[derive(Clone, Debug)]
pub struct OutputConfig {
    /// Output format
    pub format: OutputFormat,
    /// Output file path
    pub file: Option<String>,
}

/// Prints formatted output for a list of items.
pub fn print_output<T: Serialize + TableRow>(data: &[T], config: &OutputConfig) {
    if let Some(ref path) = config.file {
        if let Err(e) = write_output_to_file(data, config.format, path) {
            eprintln!("{}: {}", "File write error".red(), e);
        }
    } else {
        match config.format {
            OutputFormat::JSON => print_json(data),
            OutputFormat::Table => print_table::<T>(data),
            OutputFormat::CSV => print_csv::<T>(data),
        }
    }
}

/// Prints formatted output for a single item.
pub fn print_single<T: Serialize + SingleDisplay>(data: &T, config: &OutputConfig) {
    if let Some(ref path) = config.file {
        if let Err(e) = write_single_to_file(data, config.format, path) {
            eprintln!("{}: {}", "File write error".red(), e);
        }
    } else {
        match config.format {
            OutputFormat::JSON => {
                if let Ok(json) = serde_json::to_string_pretty(data) {
                    println!("{}", json);
                }
            }
            OutputFormat::Table | OutputFormat::CSV => data.print_single(),
        }
    }
}

fn write_output_to_file<T: Serialize + TableRow>(
    data: &[T],
    format: OutputFormat,
    path: &str,
) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    match format {
        OutputFormat::JSON => {
            let json = serde_json::to_string_pretty(data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writeln!(writer, "{}", json)?;
        }
        OutputFormat::Table => {
            if data.is_empty() {
                writeln!(writer, "No data available.")?;
            } else {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic);
                table.set_header(T::headers());
                for item in data {
                    table.add_row(item.row());
                }
                writeln!(writer, "{}", table)?;
            }
        }
        OutputFormat::CSV => {
            writer.write_all(&[0xEF, 0xBB, 0xBF])?;
            let mut csv_writer = csv::Writer::from_writer(writer);
            for item in data {
                csv_writer
                    .serialize(item)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            }
            csv_writer.flush()?;
            eprintln!("{} {}", "Saved to:".green(), path);
            return Ok(());
        }
    }

    writer.flush()?;
    eprintln!("{} {}", "Saved to:".green(), path);
    Ok(())
}

fn write_single_to_file<T: Serialize>(
    data: &T,
    format: OutputFormat,
    path: &str,
) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    match format {
        OutputFormat::JSON => {
            let json = serde_json::to_string_pretty(data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writeln!(writer, "{}", json)?;
        }
        OutputFormat::Table | OutputFormat::CSV => {
            let json = serde_json::to_string_pretty(data)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writeln!(writer, "{}", json)?;
        }
    }

    writer.flush()?;
    eprintln!("{} {}", "Saved to:".green(), path);
    Ok(())
}

fn print_json<T: Serialize>(data: &[T]) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        println!("{}", json);
    }
}

fn print_table<T: TableRow>(data: &[T]) {
    if data.is_empty() {
        println!("{}", "No data available.".yellow());
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(T::headers());

    for item in data {
        table.add_row(item.row());
    }

    println!("{table}");
}

fn print_csv<T: TableRow + Serialize>(data: &[T]) {
    if data.is_empty() {
        return;
    }

    let stdout = io::stdout();
    let mut writer = csv::Writer::from_writer(stdout.lock());

    for item in data {
        if let Err(e) = writer.serialize(item) {
            eprintln!("{}: {}", "CSV write error".red(), e);
            return;
        }
    }

    if let Err(e) = writer.flush() {
        eprintln!("{}: {}", "CSV flush error".red(), e);
    }
}

/// Trait for items displayable as table rows.
pub trait TableRow {
    fn headers() -> Vec<Cell>;
    fn row(&self) -> Vec<Cell>;
}

/// Trait for single item display.
pub trait SingleDisplay {
    fn print_single(&self);
}

/// Formats volume with unit suffix.
pub fn format_volume(vol: u64) -> String {
    if vol >= 100_000_000 {
        format!("{} ({:.2}亿)", vol, vol as f64 / 100_000_000.0)
    } else if vol >= 10_000 {
        format!("{} ({:.2}万)", vol, vol as f64 / 10_000.0)
    } else {
        format!("{}", vol)
    }
}

/// Formats amount with unit suffix.
pub fn format_amount(amount: f64) -> String {
    if amount >= 100_000_000.0 {
        format!("{:.0} ({:.2}亿)", amount, amount / 100_000_000.0)
    } else if amount >= 10_000.0 {
        format!("{:.0} ({:.2}万)", amount, amount / 10_000.0)
    } else {
        format!("{:.2}", amount)
    }
}

/// Formats change percentage with sign.
pub fn format_change_pct(pct: f64) -> String {
    if pct >= 0.0 {
        format!("+{:.2}%", pct)
    } else {
        format!("{:.2}%", pct)
    }
}

/// Creates a colored cell for change percentage.
pub fn change_pct_cell(pct: f64) -> Cell {
    let text = format_change_pct(pct);
    let cell = Cell::new(&text).set_alignment(CellAlignment::Right);
    if pct > 0.0 {
        cell.fg(Color::Red)
    } else if pct < 0.0 {
        cell.fg(Color::Green)
    } else {
        cell
    }
}

/// Creates a right-aligned cell.
pub fn right_cell<T: std::fmt::Display>(value: T) -> Cell {
    Cell::new(value.to_string()).set_alignment(CellAlignment::Right)
}

/// Creates a price cell with 2 decimal places.
pub fn price_cell(value: f64) -> Cell {
    Cell::new(format!("{:.2}", value)).set_alignment(CellAlignment::Right)
}

/// Creates a price cell with 3 decimal places.
pub fn price_cell_3(value: f64) -> Cell {
    Cell::new(format!("{:.3}", value)).set_alignment(CellAlignment::Right)
}
