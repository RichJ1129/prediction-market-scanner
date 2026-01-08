use crate::models::{Market, Position, ResolvedPosition, Trade, WalletPerformance};
use std::collections::HashMap;

/// Analyzes wallet trading performance
pub struct WalletAnalyzer;

impl WalletAnalyzer {
    /// Creates a new wallet analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyzes a wallet's trading performance
    pub fn analyze(&self, trades: &[Trade], resolved_markets: &[Market]) -> WalletPerformance {
        if trades.is_empty() {
            return self.empty_performance(String::new());
        }

        let wallet_address = trades[0].proxy_wallet.clone();

        // Build positions from trades
        let positions = self.build_positions(trades);

        // Match positions with resolved markets
        let resolved_positions = self.match_resolved_positions(&positions, resolved_markets);

        // Calculate performance metrics
        self.calculate_performance(&wallet_address, trades, &resolved_positions)
    }

    /// Builds positions from a list of trades
    fn build_positions(&self, trades: &[Trade]) -> Vec<Position> {
        let mut position_map: HashMap<(String, usize), Position> = HashMap::new();

        for trade in trades {
            let key = (trade.condition_id.clone(), trade.outcome_index);

            let position = position_map.entry(key.clone()).or_insert_with(|| Position {
                condition_id: trade.condition_id.clone(),
                outcome_index: trade.outcome_index,
                net_shares: 0.0,
                avg_price: 0.0,
                total_invested: 0.0,
                market_title: trade.title.clone().unwrap_or_else(|| "Unknown".to_string()),
            });

            match trade.side.as_str() {
                "BUY" => {
                    // Add to position
                    let new_total_shares = position.net_shares + trade.size;
                    let new_total_invested = position.total_invested + (trade.size * trade.price);

                    position.net_shares = new_total_shares;
                    position.total_invested = new_total_invested;

                    if new_total_shares > 0.0 {
                        position.avg_price = new_total_invested / new_total_shares;
                    }
                }
                "SELL" => {
                    // Reduce position
                    position.net_shares -= trade.size;
                    // On sell, we reduce the total invested proportionally
                    if position.net_shares > 0.0 {
                        position.total_invested -= trade.size * position.avg_price;
                    } else {
                        // Position closed
                        position.total_invested = 0.0;
                        position.avg_price = 0.0;
                    }
                }
                _ => {}
            }
        }

        position_map
            .into_values()
            .filter(|p| p.net_shares.abs() > 0.001) // Filter out essentially closed positions
            .collect()
    }

    /// Matches positions with resolved markets to determine wins/losses
    fn match_resolved_positions(
        &self,
        positions: &[Position],
        resolved_markets: &[Market],
    ) -> Vec<ResolvedPosition> {
        let mut resolved_positions = Vec::new();

        // Create a map of condition_id -> Market for quick lookup
        let market_map: HashMap<String, &Market> = resolved_markets
            .iter()
            .filter_map(|m| {
                m.condition_id
                    .as_ref()
                    .map(|id| (id.clone(), m))
            })
            .collect();

        for position in positions {
            if let Some(market) = market_map.get(&position.condition_id) {
                if let Some(winning_index) = self.get_winning_outcome(market) {
                    let won = position.outcome_index == winning_index;
                    let payout = if won { position.net_shares } else { 0.0 };
                    let profit = payout - position.total_invested;

                    resolved_positions.push(ResolvedPosition {
                        condition_id: position.condition_id.clone(),
                        market_title: market.question.clone(),
                        bet_outcome_index: position.outcome_index,
                        winning_outcome_index: winning_index,
                        net_shares: position.net_shares,
                        avg_price: position.avg_price,
                        total_invested: position.total_invested,
                        payout,
                        profit,
                        won,
                    });
                }
            }
        }

        resolved_positions
    }

    /// Determines the winning outcome from a market's outcome prices
    /// Returns None if market is not resolved or outcome is ambiguous
    fn get_winning_outcome(&self, market: &Market) -> Option<usize> {
        let prices_str = market.outcome_prices.as_ref()?;

        // Parse outcome prices - format: "[\"0.xxx\", \"1.xxx\"]"
        let prices: Vec<f64> = serde_json::from_str::<Vec<String>>(prices_str)
            .ok()?
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        if prices.len() != 2 {
            return None;
        }

        // The winning outcome has a price close to 1.0
        // The losing outcome has a price close to 0.0
        if prices[0] > 0.9 {
            Some(0)
        } else if prices[1] > 0.9 {
            Some(1)
        } else {
            // Market not yet resolved or ambiguous
            None
        }
    }

    /// Calculates overall performance metrics
    fn calculate_performance(
        &self,
        wallet_address: &str,
        trades: &[Trade],
        resolved_positions: &[ResolvedPosition],
    ) -> WalletPerformance {
        let total_trades = trades.len();

        // Count unique markets
        let unique_markets: std::collections::HashSet<_> =
            trades.iter().map(|t| &t.condition_id).collect();
        let total_markets = unique_markets.len();

        let resolved_positions_count = resolved_positions.len();
        let wins = resolved_positions.iter().filter(|p| p.won).count();
        let losses = resolved_positions_count - wins;

        let win_rate = if resolved_positions_count > 0 {
            (wins as f64 / resolved_positions_count as f64) * 100.0
        } else {
            0.0
        };

        let total_invested: f64 = resolved_positions.iter().map(|p| p.total_invested).sum();
        let total_payout: f64 = resolved_positions.iter().map(|p| p.payout).sum();
        let net_profit = total_payout - total_invested;

        let roi = if total_invested > 0.0 {
            (net_profit / total_invested) * 100.0
        } else {
            0.0
        };

        let winning_positions: Vec<_> = resolved_positions.iter().filter(|p| p.won).collect();
        let losing_positions: Vec<_> = resolved_positions.iter().filter(|p| !p.won).collect();

        let avg_profit_per_win = if !winning_positions.is_empty() {
            winning_positions.iter().map(|p| p.profit).sum::<f64>() / winning_positions.len() as f64
        } else {
            0.0
        };

        let avg_loss_per_loss = if !losing_positions.is_empty() {
            losing_positions.iter().map(|p| p.profit).sum::<f64>() / losing_positions.len() as f64
        } else {
            0.0
        };

        WalletPerformance {
            wallet_address: wallet_address.to_string(),
            total_trades,
            total_markets,
            resolved_positions: resolved_positions_count,
            wins,
            losses,
            win_rate,
            total_invested,
            total_payout,
            net_profit,
            roi,
            avg_profit_per_win,
            avg_loss_per_loss,
        }
    }

    /// Creates an empty performance object
    fn empty_performance(&self, wallet_address: String) -> WalletPerformance {
        WalletPerformance {
            wallet_address,
            total_trades: 0,
            total_markets: 0,
            resolved_positions: 0,
            wins: 0,
            losses: 0,
            win_rate: 0.0,
            total_invested: 0.0,
            total_payout: 0.0,
            net_profit: 0.0,
            roi: 0.0,
            avg_profit_per_win: 0.0,
            avg_loss_per_loss: 0.0,
        }
    }

    /// Determines if a wallet's performance is suspicious (potential insider)
    pub fn is_suspicious(&self, performance: &WalletPerformance) -> (bool, Vec<String>) {
        let mut flags = Vec::new();

        // Minimum thresholds to avoid false positives from small sample sizes
        if performance.resolved_positions < 10 {
            return (false, vec!["Insufficient data (less than 10 resolved positions)".to_string()]);
        }

        // Flag 1: Abnormally high win rate (>75% is very suspicious)
        if performance.win_rate > 75.0 {
            flags.push(format!(
                "Extremely high win rate: {:.1}% (normal is ~50-60%)",
                performance.win_rate
            ));
        } else if performance.win_rate > 65.0 {
            flags.push(format!(
                "Suspicious win rate: {:.1}% (normal is ~50-60%)",
                performance.win_rate
            ));
        }

        // Flag 2: High ROI with significant capital deployed
        if performance.roi > 50.0 && performance.total_invested > 1000.0 {
            flags.push(format!(
                "Very high ROI: {:.1}% with ${:.2} invested",
                performance.roi, performance.total_invested
            ));
        }

        // Flag 3: Consistent profitability across many markets
        if performance.wins > 15 && performance.win_rate > 70.0 {
            flags.push(format!(
                "Consistent high performance: {} wins out of {} resolved positions",
                performance.wins, performance.resolved_positions
            ));
        }

        // Flag 4: Large average win compared to average loss (asymmetric betting pattern)
        if performance.avg_profit_per_win > performance.avg_loss_per_loss.abs() * 2.0
            && performance.wins > 10
        {
            flags.push(format!(
                "Asymmetric profit pattern: avg win ${:.2} vs avg loss ${:.2}",
                performance.avg_profit_per_win, performance.avg_loss_per_loss
            ));
        }

        let is_suspicious = !flags.is_empty();

        (is_suspicious, flags)
    }

    /// Prints wallet performance in a formatted way
    pub fn print_performance(&self, performance: &WalletPerformance) {
        println!("\n{}", "=".repeat(80));
        println!("WALLET PERFORMANCE REPORT");
        println!("{}", "=".repeat(80));
        println!("\nWallet: {}", performance.wallet_address);
        println!("\n--- Trading Activity ---");
        println!("Total Trades:         {}", performance.total_trades);
        println!("Unique Markets:       {}", performance.total_markets);
        println!("Resolved Positions:   {}", performance.resolved_positions);

        println!("\n--- Win/Loss Record ---");
        println!("Wins:                 {}", performance.wins);
        println!("Losses:               {}", performance.losses);
        println!("Win Rate:             {:.1}%", performance.win_rate);

        println!("\n--- Financial Performance ---");
        println!("Total Invested:       ${:.2}", performance.total_invested);
        println!("Total Payout:         ${:.2}", performance.total_payout);
        println!("Net Profit:           ${:.2}", performance.net_profit);
        println!("ROI:                  {:.1}%", performance.roi);
        println!(
            "Avg Profit per Win:   ${:.2}",
            performance.avg_profit_per_win
        );
        println!(
            "Avg Loss per Loss:    ${:.2}",
            performance.avg_loss_per_loss
        );

        // Check if suspicious
        let (is_suspicious, flags) = self.is_suspicious(performance);

        if is_suspicious {
            println!("\n{}", "=".repeat(80));
            println!("⚠️  SUSPICIOUS ACTIVITY DETECTED");
            println!("{}", "=".repeat(80));
            for flag in flags {
                println!("• {}", flag);
            }
            println!("\nThis wallet shows patterns consistent with potential insider knowledge.");
            println!("{}", "=".repeat(80));
        } else {
            println!("\n{}", "-".repeat(80));
            println!("No suspicious patterns detected.");
            println!("{}", "-".repeat(80));
        }
    }
}

impl Default for WalletAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
