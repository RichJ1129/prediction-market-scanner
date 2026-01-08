use serde::Deserialize;

/// Represents a market from the Polymarket API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub question: String,
    #[serde(default)]
    pub outcome_prices: Option<String>,
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub liquidity: Option<String>,
}

/// Represents a detected arbitrage opportunity
#[derive(Debug)]
pub struct ArbitrageOpportunity {
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub total_cost: f64,
    pub profit_per_dollar: f64,
    pub profit_percent: f64,
    pub volume: f64,
    pub liquidity: f64,
}

impl ArbitrageOpportunity {
    /// Creates a new arbitrage opportunity from a market
    pub fn from_market(market: &Market, yes_price: f64, no_price: f64) -> Self {
        let total_cost = yes_price + no_price;
        let profit_per_dollar = 1.0 - total_cost;
        let profit_percent = (profit_per_dollar / total_cost) * 100.0;

        let volume: f64 = market
            .volume
            .as_ref()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);

        let liquidity: f64 = market
            .liquidity
            .as_ref()
            .and_then(|l| l.parse().ok())
            .unwrap_or(0.0);

        Self {
            question: market.question.clone(),
            yes_price,
            no_price,
            total_cost,
            profit_per_dollar,
            profit_percent,
            volume,
            liquidity,
        }
    }

    /// Prints this opportunity in a formatted way
    pub fn print(&self, index: usize) {
        println!("\n{}. {}", index, self.question);
        println!(
            "   YES: ${:.4} | NO: ${:.4} | Total: ${:.4}",
            self.yes_price, self.no_price, self.total_cost
        );
        println!(
            "   Profit: ${:.4} per $1 ({:.2}%)",
            self.profit_per_dollar, self.profit_percent
        );
        println!(
            "   Volume: ${:.2} | Liquidity: ${:.2}",
            self.volume, self.liquidity
        );
        println!("{}", "-".repeat(80));
    }
}
