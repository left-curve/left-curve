use crate::{entities::perps_pair_price::PerpsPairPrice, error::Result};
#[cfg(feature = "async-graphql")]
use {
    crate::context::Context,
    crate::entities::graphql_decimal::GraphqlBigDecimal,
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::BigDecimal,
    bigdecimal::num_bigint::BigInt,
    chrono::{Duration, Utc},
    clickhouse::Row,
    serde::Deserialize,
};

/// Helper struct for fetching a single close price from ClickHouse.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Row, Deserialize)]
struct CloseRow {
    close: u128,
}

/// Helper struct for fetching volume sum from ClickHouse.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Row, Deserialize)]
struct VolumeRow {
    total_volume: u128,
}

/// Represents 24h statistics for a perps trading pair.
/// Fields are fetched lazily when requested via GraphQL.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PerpsPairStats"))]
pub struct PerpsPairStats {
    #[cfg_attr(feature = "async-graphql", graphql(name = "pairId"))]
    pub pair_id: String,
}

impl PerpsPairStats {
    /// Creates a new PerpsPairStats for the given pair.
    pub fn new(pair_id: String) -> Self {
        Self { pair_id }
    }

    /// Fetches all perps trading pairs.
    pub async fn fetch_all(clickhouse_client: &clickhouse::Client) -> Result<Vec<Self>> {
        let pair_ids = PerpsPairPrice::all_pair_ids(clickhouse_client).await?;

        let results = pair_ids
            .into_iter()
            .map(PerpsPairStats::new)
            .collect();

        Ok(results)
    }
}

#[cfg(feature = "async-graphql")]
impl PerpsPairStats {
    /// Checks if the pair exists in the database.
    pub async fn exists(
        clickhouse_client: &clickhouse::Client,
        pair_id: &str,
    ) -> Result<bool> {
        #[derive(Debug, Row, Deserialize)]
        struct ExistsRow {
            #[allow(dead_code)]
            exists: u8,
        }

        let query = r#"
            SELECT 1 as exists
            FROM perps_pair_prices
            WHERE pair_id = ?
            LIMIT 1
        "#;

        let exists: Option<ExistsRow> = clickhouse_client
            .query(query)
            .bind(pair_id)
            .fetch_optional()
            .await?;

        Ok(exists.is_some())
    }

    /// Fetches the current (latest) close price for the pair.
    async fn fetch_current_price(
        clickhouse_client: &clickhouse::Client,
        pair_id: &str,
    ) -> Result<Option<u128>> {
        let query = r#"
            SELECT close
            FROM perps_pair_prices
            WHERE pair_id = ?
            ORDER BY block_height DESC
            LIMIT 1
        "#;

        let result: Option<CloseRow> = clickhouse_client
            .query(query)
            .bind(pair_id)
            .fetch_optional()
            .await?;

        Ok(result.map(|row| row.close))
    }

    /// Fetches the close price from ~24h ago for the pair.
    async fn fetch_price_24h_ago(
        clickhouse_client: &clickhouse::Client,
        pair_id: &str,
    ) -> Result<Option<u128>> {
        let time_24h_ago = Utc::now() - Duration::hours(24);

        let query = r#"
            SELECT close
            FROM perps_pair_prices
            WHERE pair_id = ?
              AND created_at <= toDateTime64(?, 6)
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let result: Option<CloseRow> = clickhouse_client
            .query(query)
            .bind(pair_id)
            .bind(time_24h_ago.timestamp())
            .fetch_optional()
            .await?;

        // If no price from 24h ago, use the earliest available price
        if let Some(row) = result {
            return Ok(Some(row.close));
        }

        // Get the earliest price if no data from 24h ago
        let earliest_query = r#"
            SELECT close
            FROM perps_pair_prices
            WHERE pair_id = ?
            ORDER BY block_height ASC
            LIMIT 1
        "#;

        let earliest: Option<CloseRow> = clickhouse_client
            .query(earliest_query)
            .bind(pair_id)
            .fetch_optional()
            .await?;

        Ok(earliest.map(|row| row.close))
    }

    /// Fetches the 24h volume in USD for the pair.
    /// Uses `volume_usd` from `perps_pair_prices` which is already aggregated per block.
    async fn fetch_volume_24h(
        clickhouse_client: &clickhouse::Client,
        pair_id: &str,
    ) -> Result<u128> {
        let time_24h_ago = Utc::now() - Duration::hours(24);

        let query = r#"
            SELECT sum(volume_usd) as total_volume
            FROM perps_pair_prices
            WHERE pair_id = ?
              AND created_at >= toDateTime64(?, 6)
        "#;

        let result: Option<VolumeRow> = clickhouse_client
            .query(query)
            .bind(pair_id)
            .bind(time_24h_ago.timestamp())
            .fetch_optional()
            .await?;

        Ok(result.map(|row| row.total_volume).unwrap_or(0))
    }
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PerpsPairStats {
    /// Current close price as a BigDecimal with 6 decimal places (fetched lazily)
    async fn current_price(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let price = Self::fetch_current_price(clickhouse_client, &self.pair_id).await?;

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 6).normalized().into()
        }))
    }

    /// Close price from 24 hours ago as a BigDecimal with 6 decimal places (fetched lazily)
    async fn price_24h_ago(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let price = Self::fetch_price_24h_ago(clickhouse_client, &self.pair_id).await?;

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 6).normalized().into()
        }))
    }

    /// 24h volume in USD as a BigDecimal with 6 decimal places (fetched lazily)
    async fn volume_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<GraphqlBigDecimal> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let volume = Self::fetch_volume_24h(clickhouse_client, &self.pair_id).await?;

        let bigint = BigInt::from(volume);
        Ok(BigDecimal::new(bigint, 6).normalized().into())
    }

    /// 24h price change as a percentage (e.g., 5.25 means +5.25%)
    /// Calculated as: (current_price - price_24h_ago) / price_24h_ago * 100
    /// Fetches both prices lazily.
    async fn price_change_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let current_price =
            Self::fetch_current_price(clickhouse_client, &self.pair_id).await?;
        let price_24h_ago =
            Self::fetch_price_24h_ago(clickhouse_client, &self.pair_id).await?;

        let (current, old) = match (current_price, price_24h_ago) {
            (Some(c), Some(o)) => (c, o),
            _ => return Ok(None),
        };

        if old == 0 {
            return Ok(None);
        }

        // Calculate price change percentage using BigDecimal for precision
        let current_bd = BigDecimal::new(BigInt::from(current), 6);
        let old_bd = BigDecimal::new(BigInt::from(old), 6);

        // (current - old) / old * 100
        let change = (current_bd - &old_bd) / old_bd * BigDecimal::from(100);
        Ok(Some(change.normalized().into()))
    }
}
