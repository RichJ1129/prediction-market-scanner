use crate::client::PolymarketClient;
use crate::wallet_analyzer::WalletAnalyzer;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration};

/// Scans for wallets with suspicious trading patterns
pub struct WalletScanner {
    client: PolymarketClient,
    analyzer: WalletAnalyzer,
}

impl WalletScanner {
    pub fn new() -> Self {
        Self {
            client: PolymarketClient::new(),
            analyzer: WalletAnalyzer::new(),
        }
    }

    /// Scans recent trades to find wallets with high activity
    pub async fn find_active_wallets(&self, sample_size: usize, max_wallets: usize) -> Result<Vec<String>> {
        println!("🔍 Scanning recent trades to find active wallets...");
        println!("  Fetching {} recent trades...", sample_size);

        let trades = self.client.fetch_recent_trades(sample_size).await?;

        println!("✓ Fetched {} trades", trades.len());
        println!("  Analyzing wallet activity...");

        let mut wallet_trade_count: HashMap<String, usize> = HashMap::new();

        for trade in trades {
            *wallet_trade_count.entry(trade.proxy_wallet).or_insert(0) += 1;
        }

        // Get top wallets by trade count
        let mut wallet_counts: Vec<_> = wallet_trade_count.into_iter().collect();
        wallet_counts.sort_by(|a, b| b.1.cmp(&a.1));

        println!("✓ Found {} unique wallets", wallet_counts.len());

        // Take top N wallets with at least 3 trades
        let top_wallets: Vec<String> = wallet_counts
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .take(max_wallets)
            .map(|(wallet, count)| {
                println!("  {} ({} trades)", wallet, count);
                wallet
            })
            .collect();

        println!("\n✓ Selected {} wallets for analysis\n", top_wallets.len());

        Ok(top_wallets)
    }

    /// Scans multiple wallets and identifies profitable ones
    pub async fn scan_for_insiders(&self, wallet_addresses: &[String]) -> Result<()> {
        println!("🎯 Scanning {} wallets for profitable traders...\n", wallet_addresses.len());

        // Fetch all resolved markets once (to avoid re-fetching for each wallet)
        println!("📚 Loading resolved markets database...");
        let start = std::time::Instant::now();
        let resolved_markets = self.client.fetch_resolved_markets().await?;
        println!("✓ Loaded {} resolved markets in {:.1}s\n", resolved_markets.len(), start.elapsed().as_secs_f64());

        let mut profitable_wallets = Vec::new();
        let wallet_count = wallet_addresses.len();

        for (index, wallet) in wallet_addresses.iter().enumerate() {
            print!("\r[{}/{}] Analyzing wallets...", index + 1, wallet_count);
            std::io::Write::flush(&mut std::io::stdout()).ok();

            match self.client.fetch_wallet_trades(wallet).await {
                Ok(trades) => {
                    if trades.is_empty() {
                        continue;
                    }

                    // Extract username from trades (prefer name over pseudonym)
                    let username = trades.iter()
                        .find_map(|t| t.name.as_ref().or(t.pseudonym.as_ref()))
                        .cloned();

                    let performance = self.analyzer.analyze(&trades, &resolved_markets);

                    // Filter for genuinely profitable wallets
                    // Require: 10+ resolved positions, ROI > 10%, net profit > $50
                    if performance.resolved_positions >= 10
                        && performance.roi > 10.0
                        && performance.net_profit > 50.0 {
                        let flags = self.analyzer.is_suspicious(&performance).1;
                        profitable_wallets.push((wallet.clone(), username, performance, flags));
                    }
                }
                Err(_e) => {
                    // Silently skip errors during batch processing
                }
            }
        }

        println!(); // New line after progress indicator

        // Print summary
        println!("\n{}", "=".repeat(80));
        println!("SCAN SUMMARY");
        println!("{}", "=".repeat(80));
        println!("\nScanned wallets: {}", wallet_addresses.len());
        println!("Profitable wallets found: {}\n", profitable_wallets.len());

        if !profitable_wallets.is_empty() {
            // Sort by ROI descending
            profitable_wallets.sort_by(|a, b| b.2.roi.partial_cmp(&a.2.roi).unwrap());

            println!("{}", "=".repeat(80));
            println!("PROFITABLE WALLETS (SORTED BY ROI)");
            println!("{}", "=".repeat(80));

            for (i, (wallet, username, perf, flags)) in profitable_wallets.iter().enumerate() {
                // Display wallet with username if available
                if let Some(user) = username {
                    println!("\n{}. {} (@{})", i + 1, wallet, user);
                } else {
                    println!("\n{}. {}", i + 1, wallet);
                }

                println!("   Win Rate: {:.1}% | ROI: {:.1}% | Resolved Positions: {}",
                    perf.win_rate, perf.roi, perf.resolved_positions);
                println!("   Total Invested: ${:.2} | Net Profit: ${:.2}",
                    perf.total_invested, perf.net_profit);

                if !flags.is_empty() {
                    println!("   ⚠️  Red Flags:");
                    for flag in flags {
                        println!("     • {}", flag);
                    }
                }
            }

            println!("\n{}", "=".repeat(80));
        }

        Ok(())
    }

    /// Continuously scans for profitable wallets, accumulating results over time
    pub async fn continuous_scan(&self, sample_size: usize, max_wallets: usize) -> Result<()> {
        let mut all_profitable_wallets = Vec::new();
        let mut scanned_wallets: HashSet<String> = HashSet::new();
        let mut scan_count = 0;

        // Setup Ctrl+C handler
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            shutdown_tx.send(()).ok();
        });

        // Load resolved markets once
        println!("📚 Loading resolved markets database...");
        let start = std::time::Instant::now();
        let resolved_markets = self.client.fetch_resolved_markets().await?;
        println!("✓ Loaded {} resolved markets in {:.1}s\n", resolved_markets.len(), start.elapsed().as_secs_f64());

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    println!("\n\n🛑 Stopping scan...\n");
                    break;
                }
                _ = async {
                    scan_count += 1;
                    println!("🔄 Scan iteration #{}", scan_count);
                    println!("{}", "=".repeat(80));

                    // Find active wallets
                    match self.find_active_wallets(sample_size, max_wallets).await {
                        Ok(wallets) => {
                            // Filter out already-scanned wallets
                            let new_wallets: Vec<String> = wallets
                                .into_iter()
                                .filter(|w| !scanned_wallets.contains(w))
                                .collect();

                            println!("✓ Found {} new wallets to analyze (skipped {} already scanned)\n",
                                new_wallets.len(),
                                max_wallets.saturating_sub(new_wallets.len()));

                            if new_wallets.is_empty() {
                                println!("All wallets in this batch were already scanned. Waiting for new activity...\n");
                            } else {
                                // Scan new wallets
                                let new_profitable = self.scan_wallets_internal(&new_wallets, &resolved_markets).await;

                                // Mark as scanned
                                for wallet in &new_wallets {
                                    scanned_wallets.insert(wallet.clone());
                                }

                                // Add to cumulative results
                                if !new_profitable.is_empty() {
                                    println!("\n✨ Found {} new profitable wallet(s) in this iteration!", new_profitable.len());
                                    all_profitable_wallets.extend(new_profitable);

                                    // Print cumulative summary
                                    self.print_cumulative_results(&all_profitable_wallets);
                                } else {
                                    println!("\n No profitable wallets found in this iteration.");
                                }
                            }

                            println!("\n📊 Total stats:");
                            println!("   Scans completed: {}", scan_count);
                            println!("   Wallets analyzed: {}", scanned_wallets.len());
                            println!("   Profitable wallets found: {}", all_profitable_wallets.len());
                            println!("\n⏳ Waiting 30 seconds before next scan... (Press Ctrl+C to stop)\n");

                            sleep(Duration::from_secs(30)).await;
                        }
                        Err(e) => {
                            println!("❌ Error finding wallets: {}\n", e);
                            sleep(Duration::from_secs(30)).await;
                        }
                    }
                } => {}
            }
        }

        // Print final results
        if !all_profitable_wallets.is_empty() {
            println!("\n{}", "=".repeat(80));
            println!("FINAL RESULTS");
            println!("{}", "=".repeat(80));
            println!("\nTotal scans: {}", scan_count);
            println!("Total wallets analyzed: {}", scanned_wallets.len());
            println!("Total profitable wallets found: {}\n", all_profitable_wallets.len());

            self.print_cumulative_results(&all_profitable_wallets);
        }

        Ok(())
    }

    /// Internal method to scan wallets and return profitable ones
    async fn scan_wallets_internal(&self, wallet_addresses: &[String], resolved_markets: &[crate::models::Market]) -> Vec<(String, Option<String>, crate::models::WalletPerformance, Vec<String>)> {
        let mut profitable_wallets = Vec::new();
        let wallet_count = wallet_addresses.len();

        for (index, wallet) in wallet_addresses.iter().enumerate() {
            print!("\r[{}/{}] Analyzing wallets...", index + 1, wallet_count);
            std::io::Write::flush(&mut std::io::stdout()).ok();

            if let Ok(trades) = self.client.fetch_wallet_trades(wallet).await {
                if !trades.is_empty() {
                    let username = trades.iter()
                        .find_map(|t| t.name.as_ref().or(t.pseudonym.as_ref()))
                        .cloned();

                    let performance = self.analyzer.analyze(&trades, resolved_markets);

                    // Filter for genuinely profitable wallets
                    // Require: 10+ resolved positions, ROI > 10%, net profit > $50
                    if performance.resolved_positions >= 10
                        && performance.roi > 10.0
                        && performance.net_profit > 50.0 {
                        let flags = self.analyzer.is_suspicious(&performance).1;
                        profitable_wallets.push((wallet.clone(), username, performance, flags));
                    }
                }
            }
        }

        println!(); // New line after progress indicator
        profitable_wallets
    }

    /// Prints cumulative results sorted by ROI
    fn print_cumulative_results(&self, profitable_wallets: &[(String, Option<String>, crate::models::WalletPerformance, Vec<String>)]) {
        let mut sorted = profitable_wallets.to_vec();
        sorted.sort_by(|a, b| b.2.roi.partial_cmp(&a.2.roi).unwrap());

        println!("\n{}", "=".repeat(80));
        println!("PROFITABLE WALLETS (SORTED BY ROI)");
        println!("{}", "=".repeat(80));

        for (i, (wallet, username, perf, flags)) in sorted.iter().enumerate().take(20) {
            if let Some(user) = username {
                println!("\n{}. {} (@{})", i + 1, wallet, user);
            } else {
                println!("\n{}. {}", i + 1, wallet);
            }

            println!("   Win Rate: {:.1}% | ROI: {:.1}% | Resolved Positions: {}",
                perf.win_rate, perf.roi, perf.resolved_positions);
            println!("   Total Invested: ${:.2} | Net Profit: ${:.2}",
                perf.total_invested, perf.net_profit);

            if !flags.is_empty() {
                println!("   ⚠️  Red Flags:");
                for flag in flags {
                    println!("     • {}", flag);
                }
            }
        }

        if sorted.len() > 20 {
            println!("\n... and {} more", sorted.len() - 20);
        }

        println!("\n{}", "=".repeat(80));
    }
}

impl Default for WalletScanner {
    fn default() -> Self {
        Self::new()
    }
}
