use crate::{entities::pair_price::PairPrice, error::Result};
#[cfg(feature = "async-graphql")]
use {
    crate::{context::Context, entities::graphql_decimal::GraphqlBigDecimal},
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::{BigDecimal, num_bigint::BigInt},
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
///
/// Fields are fetched lazily when requested via GraphQL **unless** the struct
/// was created with [`PairStats::resolved`], in which case the pre-computed
/// values are returned directly (used by the subscription cache).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PairStats"))]
pub struct PairStats {
    #[cfg_attr(feature = "async-graphql", graphql(name = "quoteDenom"))]
    pub quote_denom: String,
    #[cfg_attr(feature = "async-graphql", graphql(name = "baseDenom"))]
    pub base_denom: String,
    /// Pre-resolved current price. `Some` means the value was pre-computed.
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub cached_current_price: Option<Option<u128>>,
    /// Pre-resolved price from 24h ago. `Some` means the value was pre-computed.
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub cached_price_24h_ago: Option<Option<u128>>,
    /// Pre-resolved 24h volume. `Some` means the value was pre-computed.
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub cached_volume_24h: Option<u128>,
}

impl PairStats {
    /// Creates a new PairStats for the given trading pair (lazy – fields
    /// resolved on demand via ClickHouse).
    pub fn new(base_denom: String, quote_denom: String) -> Self {
        Self {
            base_denom,
            quote_denom,
            cached_current_price: None,
            cached_price_24h_ago: None,
            cached_volume_24h: None,
        }
    }

    /// Creates a PairStats with all fields pre-resolved (used by the
    /// subscription cache so that no ClickHouse query is needed at resolve
    /// time).
    pub fn resolved(
        base_denom: String,
        quote_denom: String,
        current_price: Option<u128>,
        price_24h_ago: Option<u128>,
        volume_24h: u128,
    ) -> Self {
        Self {
            base_denom,
            quote_denom,
            cached_current_price: Some(current_price),
            cached_price_24h_ago: Some(price_24h_ago),
            cached_volume_24h: Some(volume_24h),
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
            .bind(time_24h_ago.timestamp())
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
    /// Uses trades to capture all fills across the rolling 24h window.
    async fn fetch_volume_24h(
        clickhouse_client: &clickhouse::Client,
        base_denom: &str,
        quote_denom: &str,
    ) -> Result<u128> {
        let time_24h_ago = Utc::now() - Duration::hours(24);

        let query = r#"
            SELECT sum(filled_quote) as total_volume
            FROM trades
            WHERE base_denom = ? AND quote_denom = ?
              AND created_at >= toDateTime64(?, 6)
        "#;

        let result: Option<VolumeRow> = clickhouse_client
            .query(query)
            .bind(base_denom)
            .bind(quote_denom)
            .bind(time_24h_ago.timestamp())
            .fetch_optional()
            .await?;

        Ok(result.map(|row| row.total_volume).unwrap_or(0))
    }
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PairStats {
    /// Current price as a BigDecimal with 24 decimal places.
    async fn current_price(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let price = match self.cached_current_price {
            Some(cached) => cached,
            None => {
                let app_ctx = ctx.data::<Context>()?;
                Self::fetch_current_price(
                    app_ctx.clickhouse_client(),
                    &self.base_denom,
                    &self.quote_denom,
                )
                .await?
            },
        };

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 24).normalized().into()
        }))
    }

    /// Price from 24 hours ago as a BigDecimal with 24 decimal places.
    async fn price_24h_ago(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let price = match self.cached_price_24h_ago {
            Some(cached) => cached,
            None => {
                let app_ctx = ctx.data::<Context>()?;
                Self::fetch_price_24h_ago(
                    app_ctx.clickhouse_client(),
                    &self.base_denom,
                    &self.quote_denom,
                )
                .await?
            },
        };

        Ok(price.map(|p| {
            let bigint = BigInt::from(p);
            BigDecimal::new(bigint, 24).normalized().into()
        }))
    }

    /// 24h volume in quote asset as a BigDecimal with 6 decimal places.
    async fn volume_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<GraphqlBigDecimal> {
        let volume = match self.cached_volume_24h {
            Some(cached) => cached,
            None => {
                let app_ctx = ctx.data::<Context>()?;
                Self::fetch_volume_24h(
                    app_ctx.clickhouse_client(),
                    &self.base_denom,
                    &self.quote_denom,
                )
                .await?
            },
        };

        let bigint = BigInt::from(volume);
        Ok(BigDecimal::new(bigint, 6).normalized().into())
    }

    /// 24h price change as a percentage (e.g., 5.25 means +5.25%).
    /// Calculated as: (current_price - price_24h_ago) / price_24h_ago * 100
    async fn price_change_24h(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<GraphqlBigDecimal>> {
        let (current_price, price_24h_ago) =
            match (&self.cached_current_price, &self.cached_price_24h_ago) {
                (Some(cp), Some(p24)) => (*cp, *p24),
                _ => {
                    let app_ctx = ctx.data::<Context>()?;
                    let client = app_ctx.clickhouse_client();
                    let cp = Self::fetch_current_price(
                        client,
                        &self.base_denom,
                        &self.quote_denom,
                    )
                    .await?;
                    let p24 = Self::fetch_price_24h_ago(
                        client,
                        &self.base_denom,
                        &self.quote_denom,
                    )
                    .await?;
                    (cp, p24)
                },
            };

        let (current, old) = match (current_price, price_24h_ago) {
            (Some(c), Some(o)) => (c, o),
            _ => return Ok(None),
        };

        if old == 0 {
            return Ok(None);
        }

        let current_bd = BigDecimal::new(BigInt::from(current), 24);
        let old_bd = BigDecimal::new(BigInt::from(old), 24);

        let change = (current_bd - &old_bd) / old_bd * BigDecimal::from(100);
        Ok(Some(change.normalized().into()))
    }
}
