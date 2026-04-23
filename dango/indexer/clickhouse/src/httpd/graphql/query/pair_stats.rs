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
    async fn pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Base denom (e.g., 'BTC')")] base_denom: String,
        #[graphql(desc = "Quote denom (e.g., 'USDC')")] quote_denom: String,
    ) -> Result<Option<PairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let cache = app_ctx.pair_stats_cache.read().await;

        Ok(cache
            .stats()
            .iter()
            .find(|s| s.base_denom == base_denom && s.quote_denom == quote_denom)
            .cloned())
    }

    /// Get 24h statistics for all trading pairs.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume for each pair.
    async fn all_pair_stats(&self, ctx: &async_graphql::Context<'_>) -> Result<Vec<PairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let cache = app_ctx.pair_stats_cache.read().await;

        Ok(cache.stats().to_vec())
    }
}
