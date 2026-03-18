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

    pub async fn cleanup_old_synthetic_data(
        clickhouse_client: &clickhouse::Client,
        current_block: u64,
    ) -> Result<()> {
        if current_block < 1 {
            return Ok(());
        }

        let query = "DELETE FROM perps_pair_prices WHERE volume = 0 AND volume_usd = 0 AND block_height = ?";
        clickhouse_client
            .query(query)
            .bind(current_block - 1)
            .execute()
            .await?;

        Ok(())
    }
}
