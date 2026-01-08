use anyhow::Result;

// Declare modules (each module corresponds to a file in src/)
mod client;
mod models;
mod scanner;

// Import items from our modules
use client::PolymarketClient;
use scanner::ArbitrageScanner;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Polymarket Arbitrage Scanner");
    println!("============================\n");

    // Create API client
    let client = PolymarketClient::new();

    // Fetch all active markets
    println!("Fetching all active markets from Polymarket...\n");
    let markets = client.fetch_all_active_markets().await?;
    println!("Found {} active markets\n", markets.len());

    // Create scanner and scan for opportunities
    let scanner = ArbitrageScanner::default();
    let opportunities = scanner.scan(&markets);

    // Display results
    if opportunities.is_empty() {
        println!("No arbitrage opportunities found (threshold: total < $0.99)");
        println!("\nThis is normal - efficient markets eliminate arbitrage quickly.");
        println!("Run this periodically to catch fleeting opportunities.");
    } else {
        println!("Found {} arbitrage opportunities:\n", opportunities.len());
        println!("{}", "=".repeat(80));

        for (i, opp) in opportunities.iter().enumerate() {
            opp.print(i + 1);
        }
    }

    Ok(())
}
