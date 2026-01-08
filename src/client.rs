use anyhow::Result;
use crate::models::{Market, Trade};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;
use std::sync::Arc;

const GAMMA_API_URL: &str = "https://gamma-api.polymarket.com/markets";
const TRADES_API_URL: &str = "https://data-api.polymarket.com/trades";
const MAX_CONCURRENT_REQUESTS: usize = 20;

/// Client for interacting with the Polymarket API
#[derive(Clone)]
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

    /// Fetches all active markets from Polymarket using concurrent pagination
    pub async fn fetch_all_active_markets(&self) -> Result<Vec<Market>> {
        let limit = 100;

        // Fetch first page to check if pagination is needed
        let first_page = self.fetch_page(0, limit).await?;
        let first_page_count = first_page.len();

        // If first page is partial, we're done
        if first_page_count < limit {
            return Ok(first_page);
        }

        // Initialize for concurrent fetching
        let mut all_markets = first_page;
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let mut futures = FuturesUnordered::new();
        let mut next_offset = limit;
        let mut spawned_offsets = std::collections::HashSet::new();

        // Spawn initial batch of concurrent requests
        for i in 0..MAX_CONCURRENT_REQUESTS {
            let offset = next_offset + (i * limit);
            spawned_offsets.insert(offset);

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();

            futures.push(tokio::spawn(async move {
                let result = fetch_page_internal(&client, offset, limit).await;
                drop(permit);
                (offset, result)
            }));
        }

        next_offset += MAX_CONCURRENT_REQUESTS * limit;

        // Process results and spawn new requests dynamically
        while let Some(result) = futures.next().await {
            match result {
                Ok((_offset, Ok(markets))) => {
                    let page_count = markets.len();
                    all_markets.extend(markets);

                    // If page is full, spawn next request
                    if page_count == limit && !spawned_offsets.contains(&next_offset) {
                        spawned_offsets.insert(next_offset);

                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        let client = self.client.clone();
                        let offset = next_offset;

                        futures.push(tokio::spawn(async move {
                            let result = fetch_page_internal(&client, offset, limit).await;
                            drop(permit);
                            (offset, result)
                        }));

                        next_offset += limit;
                    }
                }
                Ok((offset, Err(e))) => {
                    eprintln!("Warning: Failed to fetch page at offset {}: {}", offset, e);
                    // Continue with other pages
                }
                Err(e) => {
                    eprintln!("Warning: Task failed: {}", e);
                    // Continue with other pages
                }
            }
        }

        Ok(all_markets)
    }

    /// Fetches a single page of markets
    async fn fetch_page(&self, offset: usize, limit: usize) -> Result<Vec<Market>> {
        fetch_page_internal(&self.client, offset, limit).await
    }

    /// Fetches all trades for a specific wallet address
    pub async fn fetch_wallet_trades(&self, wallet_address: &str) -> Result<Vec<Trade>> {
        let mut all_trades = Vec::new();
        let limit = 1000;
        let mut offset = 0;

        loop {
            let trades: Vec<Trade> = self.client
                .get(TRADES_API_URL)
                .query(&[
                    ("user", wallet_address),
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?
                .json()
                .await?;

            let count = trades.len();
            all_trades.extend(trades);

            if count < limit {
                break;
            }

            offset += limit;
        }

        Ok(all_trades)
    }

    /// Fetches recent trades (no wallet filter) to discover active wallets
    pub async fn fetch_recent_trades(&self, limit: usize) -> Result<Vec<Trade>> {
        let mut all_trades = Vec::new();
        let page_limit = 1000;
        let mut offset = 0;

        while all_trades.len() < limit {
            let fetch_limit = std::cmp::min(page_limit, limit - all_trades.len());

            let trades: Vec<Trade> = self.client
                .get(TRADES_API_URL)
                .query(&[
                    ("limit", &fetch_limit.to_string()),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?
                .json()
                .await?;

            let count = trades.len();
            all_trades.extend(trades);

            if count < fetch_limit {
                break;
            }

            offset += fetch_limit;
        }

        Ok(all_trades)
    }

    /// Fetches all closed/resolved markets
    pub async fn fetch_resolved_markets(&self) -> Result<Vec<Market>> {
        let limit = 100;

        // Fetch first page to check if pagination is needed
        let first_page = self.fetch_markets_page(0, limit, true).await?;
        let first_page_count = first_page.len();

        // If first page is partial, we're done
        if first_page_count < limit {
            return Ok(first_page);
        }

        // Initialize for concurrent fetching
        let mut all_markets = first_page;
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let mut futures = FuturesUnordered::new();
        let mut next_offset = limit;
        let mut spawned_offsets = std::collections::HashSet::new();

        // Spawn initial batch of concurrent requests
        for i in 0..MAX_CONCURRENT_REQUESTS {
            let offset = next_offset + (i * limit);
            spawned_offsets.insert(offset);

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();

            futures.push(tokio::spawn(async move {
                let result = fetch_resolved_markets_page(&client, offset, limit).await;
                drop(permit);
                (offset, result)
            }));
        }

        next_offset += MAX_CONCURRENT_REQUESTS * limit;

        // Process results and spawn new requests dynamically
        while let Some(result) = futures.next().await {
            match result {
                Ok((_offset, Ok(markets))) => {
                    let page_count = markets.len();
                    all_markets.extend(markets);

                    // If page is full, spawn next request
                    if page_count == limit && !spawned_offsets.contains(&next_offset) {
                        spawned_offsets.insert(next_offset);

                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        let client = self.client.clone();
                        let offset = next_offset;

                        futures.push(tokio::spawn(async move {
                            let result = fetch_resolved_markets_page(&client, offset, limit).await;
                            drop(permit);
                            (offset, result)
                        }));

                        next_offset += limit;
                    }
                }
                Ok((offset, Err(e))) => {
                    eprintln!("Warning: Failed to fetch page at offset {}: {}", offset, e);
                }
                Err(e) => {
                    eprintln!("Warning: Task failed: {}", e);
                }
            }
        }

        Ok(all_markets)
    }

    /// Fetches a single page of markets with optional closed filter
    async fn fetch_markets_page(&self, offset: usize, limit: usize, closed: bool) -> Result<Vec<Market>> {
        fetch_resolved_markets_page(&self.client, offset, limit).await
    }
}

/// Helper function to fetch a single page
async fn fetch_page_internal(
    client: &reqwest::Client,
    offset: usize,
    limit: usize,
) -> Result<Vec<Market>> {
    let markets: Vec<Market> = client
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

    Ok(markets)
}

/// Helper function to fetch a single page of resolved markets
async fn fetch_resolved_markets_page(
    client: &reqwest::Client,
    offset: usize,
    limit: usize,
) -> Result<Vec<Market>> {
    let response = client
        .get(GAMMA_API_URL)
        .query(&[
            ("closed", "true"),
            ("limit", &limit.to_string()),
            ("offset", &offset.to_string()),
        ])
        .send()
        .await?;

    // Check HTTP status
    if !response.status().is_success() {
        return Ok(Vec::new()); // Return empty vec for non-success status
    }

    // Get response text first to check if empty
    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(Vec::new()); // Return empty vec for empty responses
    }

    // Try to parse JSON
    match serde_json::from_str::<Vec<Market>>(&text) {
        Ok(markets) => Ok(markets),
        Err(e) => {
            // If JSON parsing fails, treat as end of data
            eprintln!("JSON decode error at offset {}: {} (treating as end of data)", offset, e);
            Ok(Vec::new())
        }
    }
}

impl Default for PolymarketClient {
    fn default() -> Self {
        Self::new()
    }
}
