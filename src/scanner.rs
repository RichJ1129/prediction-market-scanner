use crate::models::{ArbitrageOpportunity, Market};
use rayon::prelude::*;

/// Scans markets for arbitrage opportunities
#[derive(Clone)]
pub struct ArbitrageScanner {
    /// The threshold for detecting arbitrage (e.g., 0.99 means YES+NO < $0.99)
    threshold: f64,
}

impl ArbitrageScanner {
    /// Creates a new scanner with the given threshold
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Scans a list of markets and returns all arbitrage opportunities found
    pub fn scan(&self, markets: &[Market]) -> Vec<ArbitrageOpportunity> {
        // Use parallel iterator for CPU-bound scanning across multiple cores
        let mut opportunities: Vec<ArbitrageOpportunity> = markets
            .par_iter()
            .filter_map(|market| self.check_market(market))
            .collect();

        // Sort by profit percentage (highest first)
        opportunities.sort_by(|a, b| b.profit_percent.partial_cmp(&a.profit_percent).unwrap());

        opportunities
    }

    /// Checks a single market for arbitrage opportunity
    fn check_market(&self, market: &Market) -> Option<ArbitrageOpportunity> {
        let prices_str = market.outcome_prices.as_ref()?;

        // Parse outcome prices - they come as a JSON array string like "[\"0.5\",\"0.5\"]"
        let prices: Vec<f64> = serde_json::from_str::<Vec<String>>(prices_str)
            .ok()?
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        // Only consider binary markets (Yes/No)
        if prices.len() != 2 {
            return None;
        }

        let yes_price = prices[0];
        let no_price = prices[1];
        let total_cost = yes_price + no_price;

        // Check for arbitrage opportunity (total cost < threshold)
        if total_cost < self.threshold {
            Some(ArbitrageOpportunity::from_market(
                market, yes_price, no_price,
            ))
        } else {
            None
        }
    }
}

impl Default for ArbitrageScanner {
    fn default() -> Self {
        // Default threshold of 0.995 to account for slippage and execution costs
        Self::new(0.995)
    }
}
