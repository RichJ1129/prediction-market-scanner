use crate::client::PolymarketClient;
use crate::wallet_analyzer::WalletAnalyzer;
use anyhow::Result;
use std::collections::HashMap;

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

                    // Extract username from trades (if available)
                    let username = trades.iter()
                        .find_map(|t| t.username.as_ref())
                        .cloned();

                    let performance = self.analyzer.analyze(&trades, &resolved_markets);

                    if performance.resolved_positions < 5 {
                        continue;
                    }

                    // Filter for profitable wallets only (positive ROI and net profit)
                    if performance.roi > 0.0 && performance.net_profit > 0.0 {
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
}

impl Default for WalletScanner {
    fn default() -> Self {
        Self::new()
    }
}
