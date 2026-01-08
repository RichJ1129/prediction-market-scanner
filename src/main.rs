use anyhow::Result;
use chrono::Utc;
use std::time::Duration;
use tokio::time::Instant;

// Declare modules (each module corresponds to a file in src/)
mod client;
mod models;
mod scanner;
mod wallet_analyzer;

// Import items from our modules
use client::PolymarketClient;
use scanner::ArbitrageScanner;
use wallet_analyzer::WalletAnalyzer;

/// Run a single scan iteration
async fn run_single_scan(
    client: &PolymarketClient,
    scanner: &ArbitrageScanner,
) -> Result<usize> {
    let total_start = Instant::now();

    // Fetch all active markets with timing
    let fetch_start = Instant::now();
    let markets = client.fetch_all_active_markets().await?;
    let fetch_duration = fetch_start.elapsed();

    println!("✓ Fetched {} markets in {:.2}s (concurrent pagination)\n",
        markets.len(),
        fetch_duration.as_secs_f64()
    );

    // Scan for opportunities with timing
    let scan_start = Instant::now();
    let opportunities = scanner.scan(&markets);
    let scan_duration = scan_start.elapsed();

    println!("✓ Scanned markets in {:.3}s (parallel processing)\n",
        scan_duration.as_secs_f64()
    );

    // Display results
    if opportunities.is_empty() {
        println!("No arbitrage opportunities found (threshold: total < $0.995)");
        println!("\nThis is normal - efficient markets eliminate arbitrage quickly.");
        println!("Run this periodically to catch fleeting opportunities.");
    } else {
        println!("Found {} arbitrage opportunities:\n", opportunities.len());
        println!("{}", "=".repeat(80));

        for (i, opp) in opportunities.iter().enumerate() {
            opp.print(i + 1);
        }
    }

    let total_elapsed = total_start.elapsed();
    println!("\n[{}] Scan completed - Total: {:.2}s | Fetch: {:.2}s | Scan: {:.3}s",
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        total_elapsed.as_secs_f64(),
        fetch_duration.as_secs_f64(),
        scan_duration.as_secs_f64()
    );

    Ok(opportunities.len())
}

/// Analyzes a wallet's trading performance
async fn analyze_wallet(wallet_address: &str) -> Result<()> {
    println!("Polymarket Wallet Analyzer");
    println!("==========================\n");
    println!("Analyzing wallet: {}\n", wallet_address);

    let client = PolymarketClient::new();
    let analyzer = WalletAnalyzer::new();

    // Fetch wallet trades
    println!("📊 Fetching trade history...");
    let fetch_start = Instant::now();
    let trades = client.fetch_wallet_trades(wallet_address).await?;
    let fetch_duration = fetch_start.elapsed();
    println!("✓ Fetched {} trades in {:.2}s\n", trades.len(), fetch_duration.as_secs_f64());

    if trades.is_empty() {
        println!("No trades found for this wallet.");
        return Ok(());
    }

    // Fetch resolved markets
    println!("🔍 Fetching resolved markets...");
    let markets_start = Instant::now();
    let resolved_markets = client.fetch_resolved_markets().await?;
    let markets_duration = markets_start.elapsed();
    println!(
        "✓ Fetched {} resolved markets in {:.2}s\n",
        resolved_markets.len(),
        markets_duration.as_secs_f64()
    );

    // Analyze performance
    println!("📈 Analyzing performance...");
    let analysis_start = Instant::now();
    let performance = analyzer.analyze(&trades, &resolved_markets);
    let analysis_duration = analysis_start.elapsed();
    println!("✓ Analysis completed in {:.3}s", analysis_duration.as_secs_f64());

    // Print results
    analyzer.print_performance(&performance);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // If wallet address provided, run wallet analysis mode
    if args.len() > 1 {
        let wallet_address = &args[1];
        return analyze_wallet(wallet_address).await;
    }

    // Otherwise, run arbitrage scanner
    println!("Polymarket Arbitrage Scanner");
    println!("============================\n");
    println!("Usage: To analyze a wallet, run:");
    println!("  cargo run <wallet_address>\n");
    println!("Running arbitrage scanner...\n");

    // Create API client and scanner (reused across iterations)
    let client = PolymarketClient::new();
    let scanner = ArbitrageScanner::default();

    // Setup shutdown signal handler
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    // Spawn signal handler task
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        shutdown_tx.send(()).ok();
    });

    // Create 10-second polling interval
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let mut scan_count = 0u32;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                scan_count += 1;
                println!("[{}] Scan #{} starting...", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), scan_count);

                // Run scan with error handling
                match run_single_scan(&client, &scanner).await {
                    Ok(opportunities_found) => {
                        if opportunities_found > 0 {
                            println!("\n[{}] Arbitrage opportunity found! Stopping scanner.",
                                Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
                            break;
                        }
                        // Otherwise continue to next iteration
                    }
                    Err(e) => {
                        println!("[{}] ERROR (Scan #{}): {}",
                            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                            scan_count,
                            e
                        );
                        println!("Retrying in 10 seconds...\n");
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                println!("\n[{}] Shutdown signal received, exiting...", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
                println!("Goodbye!");
                break;
            }
        }
    }

    Ok(())
}
