use anyhow::Result;
use crate::models::Market;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;
use std::sync::Arc;

const GAMMA_API_URL: &str = "https://gamma-api.polymarket.com/markets";
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

impl Default for PolymarketClient {
    fn default() -> Self {
        Self::new()
    }
}
