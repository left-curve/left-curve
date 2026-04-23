use {
    crate::{context::Context, entities::perps_pair_stats::PerpsPairStats},
    async_graphql::*,
};

#[derive(Default, Debug)]
pub struct PerpsPairStatsQuery;

#[Object]
impl PerpsPairStatsQuery {
    /// Get 24h statistics for a specific perps trading pair.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume.
    async fn perps_pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Pair ID (e.g., 'perp/btcusd')")] pair_id: String,
    ) -> Result<Option<PerpsPairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let cache = app_ctx.perps_pair_stats_cache.read().await;

        Ok(cache
            .stats()
            .iter()
            .find(|s| s.pair_id == pair_id)
            .cloned())
    }

    /// Get 24h statistics for all perps trading pairs.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume for each pair.
    async fn all_perps_pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<PerpsPairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let cache = app_ctx.perps_pair_stats_cache.read().await;

        Ok(cache.stats().to_vec())
    }
}
