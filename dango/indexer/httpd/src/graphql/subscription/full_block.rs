use {
    crate::subscription_limiter::{acquire_subscription, guard_subscription_stream},
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_stream::BlockAndOutcome,
};

#[derive(Default)]
pub struct FullBlockSubscription;

#[Subscription]
impl FullBlockSubscription {
    /// Stream every finalized block in real time — the full `Block` (header +
    /// transactions) together with its `BlockOutcome` (every tx/cron outcome,
    /// hence every event) — as a single JSON scalar per block.
    ///
    /// The feed is served from an in-memory window on the validator (lowest
    /// latency, in-process). With `sinceBlockHeight` it first replays the
    /// retained blocks from that height, then streams the live tail; without it,
    /// it streams only blocks newer than the current tip. If `sinceBlockHeight`
    /// predates the retained window the subscription fails to start with a
    /// "resync required" error — reconnect with a newer height (deep history is
    /// available via the REST `/block/full/{block_height}` and
    /// `/block/full/range` routes on the indexer node, which return the same
    /// shape).
    async fn full_block<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(name = "sinceBlockHeight")] since_block_height: Option<u64>,
    ) -> Result<impl Stream<Item = async_graphql::Json<BlockAndOutcome>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;
        let stream_ctx = ctx.data::<dango_indexer_stream::Context>()?;

        // Every block matches (no filtering); project it to a JSON scalar,
        // mirroring the shape of the REST `/block/*` payloads.
        let stream = stream_ctx
            .blocks()
            .subscribe(since_block_height, |block: &BlockAndOutcome| {
                Some(async_graphql::Json(block.clone()))
            })
            .map_err(|resync| Error::new(resync.to_string()))?;

        Ok(guard_subscription_stream(stream, sub_guard))
    }
}
