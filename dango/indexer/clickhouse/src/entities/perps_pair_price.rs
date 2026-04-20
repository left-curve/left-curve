use {
    crate::error::Result,
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::Udec128_6,
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    grug_types::Timestamp,
};

#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PerpsPairPrice"))]
pub struct PerpsPairPrice {
    #[cfg_attr(feature = "async-graphql", graphql(name = "pairId"))]
    pub pair_id: String,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub high: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub low: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub close: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub volume: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub volume_usd: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    pub created_at: DateTime<Utc>,
    #[cfg_attr(feature = "async-graphql", graphql(name = "blockHeight"))]
    pub block_height: u64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PerpsPairPrice {
    /// Returns the block timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at.naive_utc()).to_rfc3339_string()
    }
}

impl PerpsPairPrice {
    pub async fn latest_prices(
        clickhouse_client: &clickhouse::Client,
        size: usize,
    ) -> Result<Vec<PerpsPairPrice>> {
        let query = r#"
    SELECT *
    FROM perps_pair_prices
    WHERE block_height IN (
      SELECT DISTINCT block_height
      FROM perps_pair_prices
      ORDER BY block_height DESC
      LIMIT ?
    )
    ORDER BY block_height ASC"#;

        Ok(clickhouse_client
            .query(query)
            .bind(size)
            .fetch_all()
            .await?)
    }

    pub async fn all_pair_ids(clickhouse_client: &clickhouse::Client) -> Result<Vec<String>> {
        let query = "SELECT DISTINCT pair_id FROM perps_pair_prices";

        let pairs: Vec<String> = clickhouse_client.query(query).fetch_all().await?;

        Ok(pairs)
    }

    /// Fetch all perps pair prices for a given pair whose `created_at >= since`,
    /// ordered by `block_height` ascending. Used at startup to rebuild the
    /// in-progress candle after an ungraceful shutdown (e.g. a panic triggered
    /// by a chain upgrade block) when the in-memory aggregation was lost
    /// before it could be flushed.
    pub async fn since(
        clickhouse_client: &clickhouse::Client,
        pair_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<PerpsPairPrice>> {
        let query = r#"
            SELECT pair_id, high, low, close, volume, volume_usd, created_at, block_height
            FROM perps_pair_prices
            WHERE pair_id = ? AND created_at >= toDateTime64(?, 6)
            ORDER BY block_height ASC
        "#;

        Ok(clickhouse_client
            .query(query)
            .bind(pair_id)
            .bind(since.timestamp_micros())
            .fetch_all()
            .await?)
    }
}
