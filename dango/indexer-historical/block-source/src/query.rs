//! The block-by-height GraphQL read surface — [`BlockQuery`].
//!
//! A single root field, `block(height)`, that reads one block straight from the
//! [`BlockSource`] in the schema context. It is **not** a projection: it answers
//! the generic "give me the block at H" independent of which `BlockSource` impl
//! (local / remote) is configured, so it lives next to the source abstraction it
//! reads — the same place as [`BlockLoader`](crate::BlockLoader) — behind the
//! `async-graphql` feature, and is merged into the schema's query root by the
//! composition root alongside the projections' query objects.
//!
//! `BlockData` (`= FullBlock`) carries no async-graphql `OutputType`, so the
//! block is returned as the canonical `JSON` scalar (`async_graphql::Json`) —
//! the whole `{ block, outcome }` payload the node serializes, the same shape
//! the `/block/full/{height}` REST route returns.

use {
    crate::BlockSource,
    async_graphql::{Context, Json, Object, Result},
    dango_indexer_historical_types::BlockData,
    std::sync::Arc,
};

/// Read surface for raw blocks: a single `block(height)` field. Merged into the
/// schema's query root by the composition root; reads the `Arc<dyn BlockSource>`
/// the httpd injects as context data.
#[derive(Default)]
pub struct BlockQuery;

#[Object]
impl BlockQuery {
    /// The full block at `height` — metadata, transactions, and execution
    /// outcome (`{ block, outcome }`) — read straight from the configured
    /// [`BlockSource`]. `null` when the source does not hold that height (below
    /// its backfill floor, or not yet ingested). Returned as the canonical
    /// `JSON` payload, since `BlockData` has no GraphQL object type of its own.
    async fn block(&self, ctx: &Context<'_>, height: u64) -> Result<Option<Json<BlockData>>> {
        let source = ctx.data::<Arc<dyn BlockSource>>()?;
        Ok(source.get(height).await?.map(Json))
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        async_graphql::{EmptyMutation, EmptySubscription, Schema},
        dango_indexer_historical_types::AnyResult,
        dango_primitives::{Block, BlockInfo, BlockOutcome, Hash256, Timestamp},
        std::collections::HashMap,
        tokio::sync::broadcast,
    };

    /// A [`BlockSource`] backed by a fixed height → block map. The rest of the
    /// trait is inert — this read surface only ever calls `get`.
    struct StubSource(HashMap<u64, BlockData>);

    #[async_trait::async_trait]
    impl BlockSource for StubSource {
        async fn run(self: Arc<Self>) -> AnyResult<()> {
            Ok(())
        }

        async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
            Ok(self.0.get(&height).cloned())
        }

        fn subscribe(&self) -> broadcast::Receiver<Arc<BlockData>> {
            broadcast::channel::<Arc<BlockData>>(1).1
        }

        async fn contiguous_frontier(&self) -> AnyResult<Option<u64>> {
            Ok(None)
        }
    }

    /// A minimal block at `height` (no txs, no cron) — enough to round-trip the
    /// `{ block, outcome }` JSON payload.
    fn block(height: u64) -> BlockData {
        BlockData {
            block: Block {
                info: BlockInfo {
                    height,
                    timestamp: Timestamp::from_nanos(0),
                    hash: Hash256::ZERO,
                },
                txs: vec![],
            },
            outcome: BlockOutcome {
                height,
                app_hash: Hash256::ZERO,
                cron_outcomes: vec![],
                tx_outcomes: vec![],
            },
        }
    }

    fn schema_with(
        blocks: HashMap<u64, BlockData>,
    ) -> Schema<BlockQuery, EmptyMutation, EmptySubscription> {
        let source: Arc<dyn BlockSource> = Arc::new(StubSource(blocks));
        Schema::build(BlockQuery, EmptyMutation, EmptySubscription)
            .data(source)
            .finish()
    }

    /// `block(height)` returns the block at that height — as the `JSON` payload
    /// the source holds — and `null` for a height it does not have.
    #[tokio::test]
    async fn block_by_height_reads_through_the_source() {
        let schema = schema_with(HashMap::from([(5, block(5))]));

        let resp = schema.execute("{ block(height: 5) }").await;
        assert!(resp.errors.is_empty(), "errors: {:#?}", resp.errors);
        let data = resp.data.into_json().unwrap();
        assert_eq!(data["block"], serde_json::to_value(block(5)).unwrap());

        // A height the source does not hold → null, no error.
        let resp = schema.execute("{ block(height: 9) }").await;
        assert!(resp.errors.is_empty(), "errors: {:#?}", resp.errors);
        assert_eq!(
            resp.data.into_json().unwrap()["block"],
            serde_json::Value::Null
        );
    }

    /// The field is exposed as `block(height: Int!): JSON` — a `BlockData`
    /// surfaced through the well-formed `JSON` scalar, never a dangling type ref.
    #[test]
    fn read_surface_is_well_formed() {
        let sdl = Schema::build(BlockQuery, EmptyMutation, EmptySubscription)
            .finish()
            .sdl();
        assert!(
            sdl.contains("block(height: Int!): JSON"),
            "block field shape:\n{sdl}"
        );
    }
}
