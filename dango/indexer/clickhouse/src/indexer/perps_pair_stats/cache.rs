use {
    crate::{entities::perps_pair_stats::PerpsPairStats, error::Result},
    clickhouse::Row,
    serde::Deserialize,
    std::collections::HashMap,
};

/// Helper struct for batch-fetching close prices keyed by pair_id.
#[derive(Debug, Row, Deserialize)]
struct PerpsPriceRow {
    pair_id: String,
    price: u128,
}

/// Helper struct for batch-fetching volumes keyed by pair_id.
#[derive(Debug, Row, Deserialize)]
struct PerpsVolumeRow {
    pair_id: String,
    total_volume: u128,
}

/// In-memory cache of pre-computed [`PerpsPairStats`] for all perps trading
/// pairs.
///
/// Refreshed once per block in `post_indexing` so that subscription consumers
/// read from memory instead of hitting ClickHouse on every notification.
#[derive(Debug, Default)]
pub struct PerpsPairStatsCache {
    stats: Vec<PerpsPairStats>,
}

impl PerpsPairStatsCache {
    pub fn stats(&self) -> &[PerpsPairStats] {
        &self.stats
    }

    /// Re-computes statistics for every perps pair using four batch queries
    /// (current prices, 24h-ago prices, earliest prices for fallback, 24h
    /// volumes) regardless of the number of pairs.
    pub async fn refresh(&mut self, clickhouse_client: &clickhouse::Client) -> Result<()> {
        let (current_prices, prices_24h_ago, earliest_prices, volumes) = tokio::try_join!(
            Self::fetch_current_prices(clickhouse_client),
            Self::fetch_prices_24h_ago(clickhouse_client),
            Self::fetch_earliest_prices(clickhouse_client),
            Self::fetch_volumes_24h(clickhouse_client),
        )?;

        let mut stats: Vec<PerpsPairStats> = current_prices
            .keys()
            .map(|pair_id| {
                let current_price = current_prices.get(pair_id).copied();
                let price_24h_ago = prices_24h_ago
                    .get(pair_id)
                    .copied()
                    .or_else(|| earliest_prices.get(pair_id).copied());
                let volume_24h = volumes.get(pair_id).copied().unwrap_or(0);

                PerpsPairStats::resolved(pair_id.clone(), current_price, price_24h_ago, volume_24h)
            })
            .collect();

        stats.sort_by(|a, b| a.pair_id.cmp(&b.pair_id));
        self.stats = stats;

        Ok(())
    }

    // -- batch helpers --------------------------------------------------------

    /// Latest close price per pair (the one with the highest block_height).
    async fn fetch_current_prices(
        client: &clickhouse::Client,
    ) -> Result<HashMap<String, u128>> {
        let rows: Vec<PerpsPriceRow> = client
            .query(
                r#"
                SELECT pair_id,
                       argMax(close, block_height) AS price
                FROM perps_pair_prices
                GROUP BY pair_id
                "#,
            )
            .fetch_all()
            .await?;

        Ok(rows.into_iter().map(|r| (r.pair_id, r.price)).collect())
    }

    /// Close price closest to (but not after) 24 h ago, per pair.
    async fn fetch_prices_24h_ago(
        client: &clickhouse::Client,
    ) -> Result<HashMap<String, u128>> {
        let ts = chrono::Utc::now() - chrono::Duration::hours(24);

        let rows: Vec<PerpsPriceRow> = client
            .query(
                r#"
                SELECT pair_id,
                       argMax(close, created_at) AS price
                FROM perps_pair_prices
                WHERE created_at <= toDateTime64(?, 6)
                GROUP BY pair_id
                "#,
            )
            .bind(ts.timestamp())
            .fetch_all()
            .await?;

        Ok(rows.into_iter().map(|r| (r.pair_id, r.price)).collect())
    }

    /// Earliest close price per pair – fallback when no data from 24 h ago.
    async fn fetch_earliest_prices(
        client: &clickhouse::Client,
    ) -> Result<HashMap<String, u128>> {
        let rows: Vec<PerpsPriceRow> = client
            .query(
                r#"
                SELECT pair_id,
                       argMin(close, block_height) AS price
                FROM perps_pair_prices
                GROUP BY pair_id
                "#,
            )
            .fetch_all()
            .await?;

        Ok(rows.into_iter().map(|r| (r.pair_id, r.price)).collect())
    }

    /// Sum of `volume_usd` from `perps_pair_prices` in the last 24 h, per pair.
    async fn fetch_volumes_24h(
        client: &clickhouse::Client,
    ) -> Result<HashMap<String, u128>> {
        let ts = chrono::Utc::now() - chrono::Duration::hours(24);

        let rows: Vec<PerpsVolumeRow> = client
            .query(
                r#"
                SELECT pair_id,
                       sum(volume_usd) AS total_volume
                FROM perps_pair_prices
                WHERE created_at >= toDateTime64(?, 6)
                GROUP BY pair_id
                "#,
            )
            .bind(ts.timestamp())
            .fetch_all()
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| (r.pair_id, r.total_volume))
            .collect())
    }
}
