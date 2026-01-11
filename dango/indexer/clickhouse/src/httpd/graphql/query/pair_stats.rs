use {
    crate::{context::Context, entities::pair_stats::PairStats},
    async_graphql::*,
};

#[derive(Default, Debug)]
pub struct PairStatsQuery;

#[Object]
impl PairStatsQuery {
    /// Get 24h statistics for a specific trading pair.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume.
    /// Fields are fetched lazily - only requested fields trigger database queries.
    async fn pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Base denom (e.g., 'BTC')")] base_denom: String,
        #[graphql(desc = "Quote denom (e.g., 'USDC')")] quote_denom: String,
    ) -> Result<Option<PairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        // Check if the pair exists before returning
        if !PairStats::exists(clickhouse_client, &base_denom, &quote_denom).await? {
            return Ok(None);
        }

        Ok(Some(PairStats::new(base_denom, quote_denom)))
    }

    /// Get 24h statistics for all trading pairs.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume for each pair.
    /// Fields are fetched lazily - only requested fields trigger database queries.
    async fn all_pair_stats(&self, ctx: &async_graphql::Context<'_>) -> Result<Vec<PairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let stats = PairStats::fetch_all(clickhouse_client).await?;

        Ok(stats)
    }
}
