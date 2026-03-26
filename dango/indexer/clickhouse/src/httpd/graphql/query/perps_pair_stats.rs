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
    /// Fields are fetched lazily - only requested fields trigger database queries.
    async fn perps_pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Pair ID (e.g., 'perp/btcusd')")] pair_id: String,
    ) -> Result<Option<PerpsPairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        // Check if the pair exists before returning
        if !PerpsPairStats::exists(clickhouse_client, &pair_id).await? {
            return Ok(None);
        }

        Ok(Some(PerpsPairStats::new(pair_id)))
    }

    /// Get 24h statistics for all perps trading pairs.
    /// Returns current price, price from 24h ago, 24h price change percentage, and 24h volume for each pair.
    /// Fields are fetched lazily - only requested fields trigger database queries.
    async fn all_perps_pair_stats(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<Vec<PerpsPairStats>> {
        let app_ctx = ctx.data::<Context>()?;
        let clickhouse_client = app_ctx.clickhouse_client();

        let stats = PerpsPairStats::fetch_all(clickhouse_client).await?;

        Ok(stats)
    }
}
