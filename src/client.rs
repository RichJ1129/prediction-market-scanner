use anyhow::Result;
use crate::models::Market;

const GAMMA_API_URL: &str = "https://gamma-api.polymarket.com/markets";

/// Client for interacting with the Polymarket API
pub struct PolymarketClient {
    client: reqwest::Client,
}

impl PolymarketClient {
    /// Creates a new Polymarket API client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetches all active markets from Polymarket using pagination
    pub async fn fetch_all_active_markets(&self) -> Result<Vec<Market>> {
        let mut all_markets = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let markets: Vec<Market> = self
                .client
                .get(GAMMA_API_URL)
                .query(&[
                    ("active", "true"),
                    ("closed", "false"),
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?
                .json()
                .await?;

            let count = markets.len();
            all_markets.extend(markets);

            // If we got fewer markets than the limit, we've reached the end
            if count < limit {
                break;
            }
            offset += limit;
        }

        Ok(all_markets)
    }
}

impl Default for PolymarketClient {
    fn default() -> Self {
        Self::new()
    }
}
