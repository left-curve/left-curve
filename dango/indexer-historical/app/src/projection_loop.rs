use {
    anyhow::bail,
    dango_indexer_historical_block_source::{BlockSource, GENESIS_HEIGHT},
    dango_indexer_historical_projection::{Committer, Projection},
    dango_indexer_historical_types::{AnyResult, BlockData, BlockDataExt},
    std::{cmp::Ordering, sync::Arc},
    tokio::sync::broadcast,
};

/// Drive a single projection: pull catch-up while behind the source's
/// frontier, then live-tail via the broadcast receiver. Transitions between
/// the two phases happen transparently — see `DESIGN.md` for the rationale.
pub async fn projection_loop(
    p: Arc<dyn Projection>,
    source: Arc<dyn BlockSource>,
    committer: Arc<dyn Committer>,
) -> AnyResult<()> {
    // Clamp to the genesis floor: the source never serves a height below
    // `GENESIS_HEIGHT`, so a `min_height` (or the trait default `0`) under it
    // would leave `get(cursor)` forever `None` while every broadcast is
    // `Greater` — a livelock where the projection never advances.
    let mut cursor = committer
        .cursor(p.id())
        .await?
        .map(|h| h + 1)
        .unwrap_or_else(|| p.min_height())
        .max(GENESIS_HEIGHT);

    #[cfg(feature = "tracing")]
    tracing::info!(projection = p.id(), cursor, "projection_loop starting");

    let mut maybe_rx = None;

    loop {
        // PHASE 1 — catch-up via pull. Keep reading until the source has
        // no block at `cursor` (i.e. we caught up to whatever it can serve
        // right now).
        while let Some(block) = source.get(cursor).await? {
            process_one(p.as_ref(), committer.as_ref(), &block).await?;
            cursor += 1;
        }

        let rx = maybe_rx.get_or_insert_with(|| source.subscribe());

        // PHASE 2 — live tail via broadcast.
        loop {
            match rx.recv().await {
                Ok(block) => match block.height().cmp(&cursor) {
                    Ordering::Less => {
                        continue;
                    },
                    Ordering::Equal => {
                        process_one(p.as_ref(), committer.as_ref(), &block).await?;
                        cursor += 1;
                    },
                    Ordering::Greater => {
                        break;
                    },
                },

                Err(broadcast::error::RecvError::Lagged(_skipped)) => {
                    // Broadcast buffer overflowed — we missed N blocks. Drop
                    // back to Phase 1 to recover via `get()` instead of
                    // racing the broadcast again.
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        projection = p.id(),
                        skipped = _skipped,
                        "broadcast lagged, falling back to catch-up"
                    );
                    break;
                },
                Err(broadcast::error::RecvError::Closed) => {
                    // All senders dropped while we're still running — this
                    // shouldn't happen with the current source lifecycle and
                    // indicates an upstream bug.
                    bail!(
                        "source broadcast closed unexpectedly for projection {}",
                        p.id()
                    );
                },
            }
        }
    }
}

/// One unit of work: open a write context, let the projection stage this
/// block's writes, then commit through the committer — ClickHouse flush +
/// ack first, then the Postgres transaction carrying the domain writes
/// together with the cursor update. See `DESIGN.md` § Commit protocol.
async fn process_one(
    p: &dyn Projection,
    committer: &dyn Committer,
    block: &BlockData,
) -> AnyResult<()> {
    let mut ctx = committer.begin(p.id()).await?;
    p.process(&mut ctx, block).await?;
    committer.commit(ctx, p.id(), block.height()).await
}
