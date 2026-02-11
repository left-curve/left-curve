use crate::{entities::pair_price::PairPrice, error::Result};
#[cfg(feature = "async-graphql")]
use {
    crate::context::Context,
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::BigDecimal,
    bigdecimal::num_bigint::BigInt,
    chrono::{Duration, Utc},
    clickhouse::Row,
    serde::Deserialize,
};

/// Helper struct for fetching a single price value from ClickHouse.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Row, Deserialize)]
struct PriceRow {
    clearing_price: u128,
}

/// Helper struct for fetching volume sum from ClickHouse.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Row, Deserialize)]
struct VolumeRow {
    total_volume: u128,
}

/// Represents 24h statistics for a trading pair.
/// Fields are fetched lazily when requested via GraphQL.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PairStats"))]
pub struct PairStats {
    #[cfg_attr(feature = "async-graphql", graphql(name = "quoteDenom"))]
    pub quote_denom: String,
    #[cfg_attr(feature = "async-graphql", graphql(name = "baseDenom"))]
    pub base_denom: String,
}

impl PairStats {
    /// Creates a new PairStats for the given trading pair.
    pub fn new(base_denom: String, quote_denom: String) -> Self {
        Self {
            base_denom,
            quote_denom,
        }
    }

    /// Fetches all trading pairs.
    pub async fn fetch_all(clickhouse_client: &clickhouse::Client) -> Result<Vec<Self>> {
        let pairs = PairPrice::all_pairs(clickhouse_client).await?;

        let results = pairs
            .into_iter()
            .map(|pair| PairStats::new(pair.base_denom.to_string(), pair.quote_denom.to_string()))
            .collect();

        Ok(results)
    }
}

#[cfg(feature = "async-graphql")]
impl PairStats {
    /// Helper struct for checking existence.
    async fn pair_exists(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<bool> {
        #[derive(Debug, Row, Deserialize)]
        struct ExistsRow {
            #[allow(dead_code)]
            exists: u8,
        }

        let query = r#"
            SELECT 1 as exists
            FROM pair_prices
            WHERE base_denom = ? AND quote_denom = ?
            LIMIT 1
        "#;

        let exists: Option<ExistsRow> = clickhouse_client
            .query(query)
            .bind(base_denom)
            .bind(quote_denom)
            .fetch_optional()
            .await?;

        Ok(exists.is_some())
    }

    /// Checks if the pair exists in the database.
    pub async fn exists(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<bool> {
        Self::pair_exists(clickhouse_client, base_denom, quote_denom).await
    }

    /// Fetches the current price for the pair.
    async fn fetch_current_price(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<Option<u128>> {
        let query = r#"
            SELECT clearing_price
            FROM pair_prices
            WHERE base_denom = ? AND quote_denom = ?
            ORDER BY block_height DESC
            LIMIT 1
        "#;

        let result: Option<PriceRow> = clickhouse_client
            .query(query)
            .bind(base_denom)
            .bind(quote_denom)
            .fetch_optional()
            .await?;

        Ok(result.map(|row| row.clearing_price))
    }

    /// Fetches the price from ~24h ago for the pair.
    async fn fetch_price_24h_ago(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<Option<u128>> {
        let time_24h_ago = Utc::now() - Duration::hours(24);

        let query = r#"
            SELECT clearing_price
            FROM pair_prices
            WHERE base_denom = ? AND quote_denom = ?
              AND created_at <= toDateTime64(?, 6)
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let result: Option<PriceRow> = clickhouse_client
            .query(query)
            .bind(base_denom)
            .bind(quote_denom)
            .bind(time_24h_ago.timestamp_micros())
            .fetch_optional()
            .await?;

        // If no price from 24h ago, use the earliest available price
        if let Some(row) = result {
            return Ok(Some(row.clearing_price));
        }

        // Get the earliest price if no data from 24h ago
        let earliest_query = r#"
            SELECT clearing_price
            FROM pair_prices
            WHERE base_denom = ? AND quote_denom = ?
            ORDER BY block_height ASC
            LIMIT 1
        "#;

        let earliest: Option<PriceRow> = clickhouse_client
            .query(earliest_query)
            .bind(base_denom)
            .bind(quote_denom)
            .fetch_optional()
            .await?;

        Ok(earliest.map(|row| row.clearing_price))
    }

    /// Fetches the 24h volume in quote asset for the pair.
    /// Uses pair_prices to include all trades in the current hour.
    async fn fetch_volume_24h(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<u128> {
        let time_24h_ago = Utc::now() - Duration::hours(24);

        let query = r#"
            SELECT sum(volume_quote) as total_volume
            FROM pair_prices
            WHERE base_denom = ? AND quote_denom = ?
              AND created_at >= toDateTime64(?, 6)
        "#;

        let result: Option<VolumeRow> = clickhouse_client
            .query(query)
            .bind(base_denom)
            .bind(quote_denom)
            .bind(time_24h_ago.timestamp_micros())
            .fetch_optional()
            .await?;

        Ok(result.map(|row| row.total_volume).unwrap_or(0))
    }
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PairStats {
    /// Current price as a BigDecimal with 24 decimal places (fetched lazily)
    async fn current_price(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<BigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let price =
            Self::fetch_current_price(clickhouse_client, &self.base_denom, &self.quote_denom)
                .await?;

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 24).normalized()
        }))
    }

    /// Price from 24 hours ago as a BigDecimal with 24 decimal places (fetched lazily)
    async fn price_24h_ago(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<BigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let price =
            Self::fetch_price_24h_ago(clickhouse_client, &self.base_denom, &self.quote_denom)
                .await?;

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 24).normalized()
        }))
    }

    /// 24h volume in quote asset as a BigDecimal with 6 decimal places (fetched lazily)
    async fn volume_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<BigDecimal> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let volume =
            Self::fetch_volume_24h(clickhouse_client, &self.base_denom, &self.quote_denom).await?;

        let bigint = BigInt::from(volume);
        Ok(BigDecimal::new(bigint, 6).normalized())
    }

    /// 24h price change as a percentage (e.g., 5.25 means +5.25%)
    /// Calculated as: (current_price - price_24h_ago) / price_24h_ago * 100
    /// Fetches both prices lazily.
    async fn price_change_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<BigDecimal>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let current_price =
            Self::fetch_current_price(clickhouse_client, &self.base_denom, &self.quote_denom)
                .await?;
        let price_24h_ago =
            Self::fetch_price_24h_ago(clickhouse_client, &self.base_denom, &self.quote_denom)
                .await?;

        let (current, old) = match (current_price, price_24h_ago) {
            (Some(c), Some(o)) => (c, o),
            _ => return Ok(None),
        };

        if old == 0 {
            return Ok(None);
        }

        // Calculate price change percentage using BigDecimal for precision
        let current_bd = BigDecimal::new(BigInt::from(current), 24);
        let old_bd = BigDecimal::new(BigInt::from(old), 24);

        // (current - old) / old * 100
        let change = (current_bd - &old_bd) / old_bd * BigDecimal::from(100);
        Ok(Some(change.normalized()))
    }
}
