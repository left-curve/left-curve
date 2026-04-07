use {
    crate::{entities::pair_stats::PairStats, error::Result},
    clickhouse::Row,
    serde::Deserialize,
    std::collections::HashMap,
};

/// Helper struct for batch-fetching prices keyed by (base_denom, quote_denom).
#[derive(Debug, Row, Deserialize)]
struct PairPriceRow {
    base_denom: String,
    quote_denom: String,
    price: u128,
}

/// Helper struct for batch-fetching volumes keyed by (base_denom, quote_denom).
#[derive(Debug, Row, Deserialize)]
struct PairVolumeRow {
    base_denom: String,
    quote_denom: String,
    total_volume: u128,
}

/// In-memory cache of pre-computed [`PairStats`] for all trading pairs.
///
/// Refreshed once per block in `post_indexing` so that subscription consumers
/// read from memory instead of hitting ClickHouse on every notification.
#[derive(Debug, Default)]
pub struct PairStatsCache {
    stats: Vec<PairStats>,
}

impl PairStatsCache {
    pub fn stats(&self) -> &[PairStats] {
        &self.stats
    }

    /// Re-computes statistics for every trading pair using four batch queries
    /// (current prices, 24h-ago prices, earliest prices for fallback, 24h
    /// volumes) regardless of the number of pairs.
    pub async fn refresh(&mut self, clickhouse_client: &clickhouse::Client) -> Result<()> {
        let (current_prices, prices_24h_ago, earliest_prices, volumes) = tokio::try_join!(
            Self::fetch_current_prices(clickhouse_client),
            Self::fetch_prices_24h_ago(clickhouse_client),
            Self::fetch_earliest_prices(clickhouse_client),
            Self::fetch_volumes_24h(clickhouse_client),
        )?;

        // Collect all known pairs from the current prices map (authoritative).
        let mut stats: Vec<PairStats> = current_prices
            .keys()
            .map(|key| {
                let current_price = current_prices.get(key).copied();
                let price_24h_ago = prices_24h_ago
                    .get(key)
                    .copied()
                    .or_else(|| earliest_prices.get(key).copied());
                let volume_24h = volumes.get(key).copied().unwrap_or(0);

                PairStats::resolved(
                    key.0.clone(),
                    key.1.clone(),
                    current_price,
                    price_24h_ago,
                    volume_24h,
                )
            })
            .collect();

        // Stable ordering for deterministic results.
        stats.sort_by(|a, b| (&a.base_denom, &a.quote_denom).cmp(&(&b.base_denom, &b.quote_denom)));
        self.stats = stats;

        Ok(())
    }

    // -- batch helpers --------------------------------------------------------

    /// Latest clearing_price per pair (the one with the highest block_height).
    async fn fetch_current_prices(
        client: &clickhouse::Client,
    ) -> Result<HashMap<(String, String), u128>> {
        let rows: Vec<PairPriceRow> = client
            .query(
                r#"
                SELECT base_denom, quote_denom,
                       argMax(clearing_price, block_height) AS price
                FROM pair_prices
                GROUP BY base_denom, quote_denom
                "#,
            )
            .fetch_all()
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ((r.base_denom, r.quote_denom), r.price))
            .collect())
    }

    /// Clearing price closest to (but not after) 24 h ago, per pair.
    async fn fetch_prices_24h_ago(
        client: &clickhouse::Client,
    ) -> Result<HashMap<(String, String), u128>> {
        let ts = chrono::Utc::now() - chrono::Duration::hours(24);

        let rows: Vec<PairPriceRow> = client
            .query(
                r#"
                SELECT base_denom, quote_denom,
                       argMax(clearing_price, created_at) AS price
                FROM pair_prices
                WHERE created_at <= toDateTime64(?, 6)
                GROUP BY base_denom, quote_denom
                "#,
            )
            .bind(ts.timestamp())
            .fetch_all()
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ((r.base_denom, r.quote_denom), r.price))
            .collect())
    }

    /// Earliest clearing_price per pair – used as fallback when no data exists
    /// from 24 h ago.
    async fn fetch_earliest_prices(
        client: &clickhouse::Client,
    ) -> Result<HashMap<(String, String), u128>> {
        let rows: Vec<PairPriceRow> = client
            .query(
                r#"
                SELECT base_denom, quote_denom,
                       argMin(clearing_price, block_height) AS price
                FROM pair_prices
                GROUP BY base_denom, quote_denom
                "#,
            )
            .fetch_all()
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ((r.base_denom, r.quote_denom), r.price))
            .collect())
    }

    /// Sum of `filled_quote` from the trades table in the last 24 h, per pair.
    async fn fetch_volumes_24h(
        client: &clickhouse::Client,
    ) -> Result<HashMap<(String, String), u128>> {
        let ts = chrono::Utc::now() - chrono::Duration::hours(24);

        let rows: Vec<PairVolumeRow> = client
            .query(
                r#"
                SELECT base_denom, quote_denom,
                       sum(filled_quote) AS total_volume
                FROM trades
                WHERE created_at >= toDateTime64(?, 6)
                GROUP BY base_denom, quote_denom
                "#,
            )
            .bind(ts.timestamp())
            .fetch_all()
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ((r.base_denom, r.quote_denom), r.total_volume))
            .collect())
    }
}
