//! Convertible bond data example.

use tenk::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== tenk Convertible Bond Data Example ===\n");

    let client = ClientBuilder::new().build()?;

    println!("1. Fetching all convertible bond data...");
    let bonds = client.get_bond_current(None).await?;
    println!("   Found {} convertible bonds\n", bonds.len());

    let active_bonds: Vec<_> = bonds
        .iter()
        .filter(|b| b.price > 0.0 && b.volume > 0)
        .collect();
    println!("2. Active bonds (with trading): {}\n", active_bonds.len());

    let mut gainers: Vec<_> = active_bonds.iter().filter(|b| b.change_pct > 0.0).collect();
    gainers.sort_by(|a, b| b.change_pct.partial_cmp(&a.change_pct).unwrap());

    println!("3. Top 10 Gainers:");
    println!(
        "   {:10} {:15} {:>10} {:>10} {:>12}",
        "Code", "Name", "Price", "Change%", "Volume"
    );
    println!("   {}", "-".repeat(62));
    for bond in gainers.iter().take(10) {
        println!(
            "   {:10} {:15} {:>10.2} {:>9.2}%↑ {:>12}",
            bond.bond_code,
            bond.bond_name,
            bond.price,
            bond.change_pct,
            format_volume(bond.volume)
        );
    }
    println!();

    let mut losers: Vec<_> = active_bonds.iter().filter(|b| b.change_pct < 0.0).collect();
    losers.sort_by(|a, b| a.change_pct.partial_cmp(&b.change_pct).unwrap());

    println!("4. Top 10 Losers:");
    println!(
        "   {:10} {:15} {:>10} {:>10} {:>12}",
        "Code", "Name", "Price", "Change%", "Volume"
    );
    println!("   {}", "-".repeat(62));
    for bond in losers.iter().take(10) {
        println!(
            "   {:10} {:15} {:>10.2} {:>9.2}%↓ {:>12}",
            bond.bond_code,
            bond.bond_name,
            bond.price,
            bond.change_pct.abs(),
            format_volume(bond.volume)
        );
    }
    println!();

    let mut by_volume = active_bonds.clone();
    by_volume.sort_by(|a, b| b.volume.cmp(&a.volume));

    println!("5. Top 10 by Volume:");
    println!(
        "   {:10} {:15} {:>10} {:>10} {:>15}",
        "Code", "Name", "Price", "Change%", "Amount(万)"
    );
    println!("   {}", "-".repeat(65));
    for bond in by_volume.iter().take(10) {
        let arrow = if bond.change_pct >= 0.0 { "↑" } else { "↓" };
        println!(
            "   {:10} {:15} {:>10.2} {:>9.2}%{} {:>15.2}",
            bond.bond_code,
            bond.bond_name,
            bond.price,
            bond.change_pct.abs(),
            arrow,
            bond.amount / 10000.0
        );
    }
    println!();

    let prices: Vec<f64> = active_bonds.iter().map(|b| b.price).collect();
    if !prices.is_empty() {
        let avg_price: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let premium_count = prices.iter().filter(|&&p| p > 100.0).count();
        let discount_count = prices.iter().filter(|&&p| p < 100.0).count();

        println!("6. Price Statistics:");
        println!("   Average price: {:.2}", avg_price);
        println!("   Price range: {:.2} - {:.2}", min_price, max_price);
        println!("   Premium (>100): {} bonds", premium_count);
        println!("   Discount (<100): {} bonds", discount_count);
    }

    println!("\n7. Querying specific bond (127046)...");
    let specific_codes = ["127046"];
    let specific_bonds = client.get_bond_current(Some(&specific_codes)).await?;

    if specific_bonds.is_empty() {
        println!("   No bond found for code: 127046");
    } else {
        for bond in &specific_bonds {
            println!(
                "   {} ({}) - Price: {:.2}, Change: {:+.2}%",
                bond.bond_code, bond.bond_name, bond.price, bond.change_pct
            );
        }
    }

    println!("\nDone!");
    Ok(())
}

fn format_volume(vol: u64) -> String {
    if vol >= 100_000_000 {
        format!("{:.2}亿", vol as f64 / 100_000_000.0)
    } else if vol >= 10_000 {
        format!("{:.2}万", vol as f64 / 10_000.0)
    } else {
        format!("{}", vol)
    }
}
