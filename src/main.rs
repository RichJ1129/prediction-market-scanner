use anyhow::Result;
use chrono::Utc;
use std::time::Duration;
use tokio::time::Instant;

// Declare modules (each module corresponds to a file in src/)
mod client;
mod models;
mod scanner;
mod wallet_analyzer;
mod wallet_scanner;

// Import items from our modules
use client::PolymarketClient;
use scanner::ArbitrageScanner;
use wallet_analyzer::WalletAnalyzer;
use wallet_scanner::WalletScanner;

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

/// Auto-scan mode: Find and analyze active wallets for insider patterns
async fn auto_scan_for_insiders(sample_size: usize, max_wallets: usize, continuous: bool) -> Result<()> {
    println!("Polymarket Insider Scanner");
    println!("==========================\n");

    if continuous {
        println!("Running in CONTINUOUS mode - Press Ctrl+C to stop");
        println!("Will keep scanning for profitable wallets and accumulate results...\n");
    } else {
        println!("Automatically finding and analyzing wallets for insider patterns...\n");
    }

    let scanner = WalletScanner::new();

    if continuous {
        scanner.continuous_scan(sample_size, max_wallets).await?;
    } else {
        // Step 1: Find active wallets
        let wallets = scanner.find_active_wallets(sample_size, max_wallets).await?;

        if wallets.is_empty() {
            println!("No active wallets found.");
            return Ok(());
        }

        // Step 2: Analyze them for insider patterns
        scanner.scan_for_insiders(&wallets).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for --scan flag
    if args.len() > 1 && args[1] == "--scan" {
        let sample_size = if args.len() > 2 {
            args[2].parse().unwrap_or(5000)
        } else {
            5000
        };
        let max_wallets = if args.len() > 3 {
            args[3].parse().unwrap_or(30)
        } else {
            30
        };
        let continuous = args.len() > 4 && args[4] == "--continuous";
        return auto_scan_for_insiders(sample_size, max_wallets, continuous).await;
    }

    // If wallet address provided, run wallet analysis mode
    if args.len() > 1 && args[1].starts_with("0x") {
        let wallet_address = &args[1];
        return analyze_wallet(wallet_address).await;
    }

    // Otherwise, run arbitrage scanner
    println!("Polymarket Analysis Tools");
    println!("=========================\n");
    println!("Usage:");
    println!("  cargo run -- --scan [sample_size] [max_wallets] [--continuous]");
    println!("                                     - Auto-scan for profitable wallets");
    println!("                                       (defaults: 5000 trades, 30 wallets)");
    println!("                                       Add --continuous to run indefinitely");
    println!("  cargo run -- <wallet_address>      - Analyze a specific wallet");
    println!("  cargo run                          - Run arbitrage scanner\n");
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
