use crate::client::PolymarketClient;
use crate::models::Trade;
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
    pub async fn find_active_wallets(&self, sample_size: usize) -> Result<Vec<String>> {
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
            .take(30)
            .map(|(wallet, count)| {
                println!("  {} ({} trades)", wallet, count);
                wallet
            })
            .collect();

        println!("\n✓ Selected {} wallets for analysis\n", top_wallets.len());

        Ok(top_wallets)
    }

    /// Scans multiple wallets and identifies suspicious ones
    pub async fn scan_for_insiders(&self, wallet_addresses: &[String]) -> Result<()> {
        println!("🎯 Scanning {} wallets for insider patterns...\n", wallet_addresses.len());

        // Fetch all resolved markets once (to avoid re-fetching for each wallet)
        println!("📚 Loading resolved markets database...");
        let resolved_markets = self.client.fetch_resolved_markets().await?;
        println!("✓ Loaded {} resolved markets\n", resolved_markets.len());

        let mut suspicious_wallets = Vec::new();

        for (index, wallet) in wallet_addresses.iter().enumerate() {
            println!("[{}/{}] Analyzing {}...", index + 1, wallet_addresses.len(), wallet);

            match self.client.fetch_wallet_trades(wallet).await {
                Ok(trades) => {
                    if trades.is_empty() {
                        println!("  No trades found.\n");
                        continue;
                    }

                    let performance = self.analyzer.analyze(&trades, &resolved_markets);

                    if performance.resolved_positions < 5 {
                        println!("  Insufficient data ({} resolved positions).\n", performance.resolved_positions);
                        continue;
                    }

                    let (is_suspicious, flags) = self.analyzer.is_suspicious(&performance);

                    if is_suspicious {
                        println!("  ⚠️  SUSPICIOUS! Win rate: {:.1}%, ROI: {:.1}%", performance.win_rate, performance.roi);
                        for flag in &flags {
                            println!("     • {}", flag);
                        }
                        suspicious_wallets.push((wallet.clone(), performance, flags));
                    } else {
                        println!("  ✓ Normal activity (Win rate: {:.1}%, ROI: {:.1}%)", performance.win_rate, performance.roi);
                    }

                    println!();
                }
                Err(e) => {
                    println!("  Error: {}\n", e);
                }
            }
        }

        // Print summary
        println!("\n{}", "=".repeat(80));
        println!("SCAN SUMMARY");
        println!("{}", "=".repeat(80));
        println!("\nScanned wallets: {}", wallet_addresses.len());
        println!("Suspicious wallets found: {}\n", suspicious_wallets.len());

        if !suspicious_wallets.is_empty() {
            println!("{}", "=".repeat(80));
            println!("SUSPICIOUS WALLETS (POTENTIAL INSIDERS)");
            println!("{}", "=".repeat(80));

            for (i, (wallet, perf, flags)) in suspicious_wallets.iter().enumerate() {
                println!("\n{}. {}", i + 1, wallet);
                println!("   Win Rate: {:.1}% | ROI: {:.1}% | Resolved Positions: {}",
                    perf.win_rate, perf.roi, perf.resolved_positions);
                println!("   Total Invested: ${:.2} | Net Profit: ${:.2}",
                    perf.total_invested, perf.net_profit);
                println!("   Red Flags:");
                for flag in flags {
                    println!("     • {}", flag);
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
