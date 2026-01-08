use anyhow::Result;
use chrono::Utc;
use std::time::Duration;
use tokio::time::Instant;

// Declare modules (each module corresponds to a file in src/)
mod client;
mod models;
mod scanner;

// Import items from our modules
use client::PolymarketClient;
use scanner::ArbitrageScanner;

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

#[tokio::main]
async fn main() -> Result<()> {
    println!("Polymarket Arbitrage Scanner");
    println!("============================\n");

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

    // Create 30-second polling interval
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut scan_count = 0u32;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                scan_count += 1;
                println!("[{}] Scan #{} starting...", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), scan_count);

                // Run scan with error handling
                match run_single_scan(&client, &scanner).await {
                    Ok(_) => {
                        // Success - continue to next iteration
                    }
                    Err(e) => {
                        println!("[{}] ERROR (Scan #{}): {}",
                            Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                            scan_count,
                            e
                        );
                        println!("Retrying in 30 seconds...\n");
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
