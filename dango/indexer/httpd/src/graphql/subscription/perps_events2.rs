use {
    crate::subscription_limiter::{acquire_subscription, guard_subscription_stream},
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_stream::{PerpsEventBlock, make_perps_filter},
    std::collections::HashSet,
};

#[derive(Default)]
pub struct PerpsEvents2Subscription;

#[Subscription]
impl PerpsEvents2Subscription {
    /// Stream perps-exchange contract events (e.g. `order_filled`, `liquidated`,
    /// `deleveraged`, `order_persisted`, `order_removed`) in real time, grouped
    /// per block.
    ///
    /// The feed is served from an in-memory window on the validator (lowest
    /// latency). It first replays recent retained blocks — from
    /// `sinceBlockHeight` if given, otherwise none — then streams the live tail.
    /// The `eventTypes`, `pairIds`, `users`, `orderIds`, and `clientOrderIds`
    /// filters AND together; for each, omitting it matches everything, while
    /// passing an empty list matches nothing. A `clientOrderId` is unique only
    /// per sender, so combine it with `users` to target a single trader's order.
    /// If `sinceBlockHeight` predates the retained window, the
    /// subscription fails to start with a "resync required" error — reconnect
    /// with a newer `sinceBlockHeight` (deep history is available via the
    /// `perpsEvents` query on the indexer node).
    async fn perps_events2<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(name = "sinceBlockHeight")] since_block_height: Option<u64>,
        #[graphql(name = "eventTypes")] event_types: Option<HashSet<String>>,
        #[graphql(name = "pairIds")] pair_ids: Option<HashSet<String>>,
        users: Option<HashSet<String>>,
        #[graphql(name = "orderIds")] order_ids: Option<HashSet<String>>,
        #[graphql(name = "clientOrderIds")] client_order_ids: Option<HashSet<String>>,
    ) -> Result<impl Stream<Item = PerpsEventBlock> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;
        let stream_ctx = ctx.data::<dango_indexer_stream::Context>()?;

        // All five filters are opaque string sets: `None` does not filter,
        // `Some(empty)` matches nothing. Each value is matched verbatim against
        // the event's canonical string form (address, denom, or decimal id) —
        // not parsed/validated — so the treatment is uniform across filters.
        let filter = make_perps_filter(event_types, pair_ids, users, order_ids, client_order_ids);

        let stream = stream_ctx
            .perps()
            .subscribe(since_block_height, filter)
            .map_err(|resync| Error::new(resync.to_string()))?;

        Ok(guard_subscription_stream(stream, sub_guard))
    }
}
