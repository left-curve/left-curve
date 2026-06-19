use {
    crate::{BlockFetcher, FetchStream},
    dango_indexer_historical_types::BlockData,
    dango_primitives::BlockClient,
    futures::future::join_all,
    std::{cmp::min, time::Duration},
    tokio::{sync::mpsc, time::sleep},
};

/// Upper bound on blocks fetched-ahead but not yet consumed. The fetcher can
/// be far faster than the store writer during a backfill; a bounded channel
/// makes a full buffer block the fetcher (backpressure) instead of letting it
/// race hundreds of thousands of blocks ahead and balloon RAM. No throughput
/// cost — the consumer is the bottleneck and the fetcher only needs a lead.
const FETCH_CHANNEL_CAPACITY: usize = 10_000;

/// Backoff after a failed or timed-out batch before retrying.
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

/// Tuning for [`SentinelBlockFetcher`].
#[derive(Debug, Clone)]
pub struct SentinelFetcherConfig {
    /// Blocks fetched concurrently per batch, clamped to the blocks left in the
    /// gap. Bounds load on the sentinel.
    pub batch_size: u64,
    /// Per-batch timeout before retrying.
    pub timeout: Duration,
}

impl Default for SentinelFetcherConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            timeout: Duration::from_secs(30),
        }
    }
}

/// [`BlockFetcher`] backed by a sentinel node's block RPC.
///
/// Fills a bounded gap `[from, to]` by fetching **fixed-size batches** of
/// heights concurrently (up to `batch_size`, clamped to the blocks left), and
/// streams the assembled [`BlockData`] in ascending order. Adapted from the
/// bots `BlockFetcher`, but simpler: that one ramps parallelism up and down
/// because it also follows the chain tip (polling once it catches up); here
/// every height in the gap is below the live tip and therefore exists, so
/// there is no tip to ramp down to — a plain fixed batch. Keeps the full
/// `Block` (the bots version drops everything but `block.info`).
///
/// Generic over the client so the crate depends only on the `BlockClient`
/// trait, not a concrete HTTP client — and so it can be driven by a mock in
/// tests.
pub struct SentinelBlockFetcher<C> {
    client: C,
    config: SentinelFetcherConfig,
}

impl<C> SentinelBlockFetcher<C> {
    pub fn new(client: C, config: SentinelFetcherConfig) -> Self {
        Self { client, config }
    }
}

impl<C> BlockFetcher for SentinelBlockFetcher<C>
where
    C: BlockClient + Clone + Send + Sync + 'static,
    C::Error: Into<anyhow::Error> + Send,
{
    fn spawn(&self, from: u64, to: u64) -> FetchStream {
        let (tx, rx) = mpsc::channel(FETCH_CHANNEL_CAPACITY);
        let handle = tokio::spawn(fetch_range(
            from,
            to,
            tx,
            self.client.clone(),
            self.config.clone(),
        ));
        FetchStream::new(rx, handle)
    }
}

/// Fetch `[from, to]` inclusive, sending each `BlockData` in ascending order
/// through `tx`. Returns when the range is done or the consumer drops the
/// receiver. Transient failures (RPC error, timeout) back off and retry from
/// the current height — every block in the range exists, so a failure is
/// always transient and never surfaced to the consumer.
async fn fetch_range<C>(
    mut height: u64,
    to: u64,
    tx: mpsc::Sender<BlockData>,
    client: C,
    config: SentinelFetcherConfig,
) where
    C: BlockClient + Clone + Send + Sync + 'static,
    C::Error: Into<anyhow::Error> + Send,
{
    let batch_size = config.batch_size.max(1);

    while height <= to {
        // Fixed batch, clamped to the blocks left in the range.
        let batch = min(batch_size, to - height + 1);

        let tasks = (0..batch)
            .map(|i| {
                let client = client.clone();
                let fetch_height = height + i;
                async move {
                    let (block, outcome) = futures::join!(
                        client.query_block(Some(fetch_height)),
                        client.query_block_outcome(Some(fetch_height)),
                    );
                    Ok::<BlockData, anyhow::Error>(BlockData {
                        block: block.map_err(Into::into)?,
                        outcome: outcome.map_err(Into::into)?,
                    })
                }
            })
            .collect::<Vec<_>>();

        let Ok(results) = tokio::time::timeout(config.timeout, join_all(tasks)).await else {
            #[cfg(feature = "tracing")]
            tracing::warn!(height, "sentinel block query timed out, retrying");
            sleep(RETRY_BACKOFF).await;
            continue;
        };

        // Results are in ascending order; send each until the first failure,
        // then back off and re-fetch from there. Anything fetched past the
        // failure is dropped and re-fetched next batch — correctness over a
        // marginal saved request.
        for result in results {
            match result {
                Ok(block) => {
                    if tx.send(block).await.is_err() {
                        // The consumer dropped the `FetchStream`; stop.
                        return;
                    }
                    height += 1;
                },
                Err(_error) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(error = %_error, height, "sentinel block query failed, retrying");
                    sleep(RETRY_BACKOFF).await;
                    break;
                },
            }
        }
    }
}
