use {
    anyhow::bail,
    indexer_historical_block_source::BlockSource,
    indexer_historical_projection::Projection,
    indexer_historical_types::AnyResult,
    std::{cmp::Ordering, sync::Arc},
    tokio::sync::broadcast,
};

/// Drive a single projection: pull catch-up while behind the source's
/// frontier, then live-tail via the broadcast receiver. Transitions between
/// the two phases happen transparently — see `DESIGN.md` for the rationale.
pub async fn projection_loop(
    p: Arc<dyn Projection>,
    source: Arc<dyn BlockSource>,
) -> AnyResult<()> {
    let mut cursor = p
        .last_processed_height()
        .await?
        .map(|h| h + 1)
        .unwrap_or_else(|| p.min_height());

    #[cfg(feature = "tracing")]
    tracing::info!(projection = p.id(), cursor, "projection_loop starting");

    let mut maybe_rx = None;

    loop {
        // PHASE 1 — catch-up via pull. Keep reading until the source has
        // no block at `cursor` (i.e. we caught up to whatever it can serve
        // right now).
        while let Some(block) = source.get(cursor).await? {
            p.process(&block).await?;
            cursor += 1;
        }

        let rx = maybe_rx.get_or_insert_with(|| source.subscribe());

        // PHASE 2 — live tail via broadcast.
        loop {
            match rx.recv().await {
                Ok(block) => match block.block.info.height.cmp(&cursor) {
                    Ordering::Less => {
                        continue;
                    },
                    Ordering::Equal => {
                        p.process(&block).await?;
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
