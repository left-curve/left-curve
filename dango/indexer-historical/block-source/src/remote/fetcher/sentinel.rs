use {
    super::{BlockFetcher, FetchStream},
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    reqwest::{IntoUrl, Url},
    std::{cmp::min, time::Duration},
    tokio::{sync::mpsc, time::sleep},
};

/// The sentinel's `/block/full/range` endpoint caps a single response at this
/// many blocks. Requesting more buys nothing — the run just continues on the
/// next request.
pub const MAX_BLOCK_RANGE: u64 = 20;

/// Fetches contiguous runs of full blocks from a sentinel's indexer-httpd
/// `/block/full/range` endpoint (`?from=&to=`).
///
/// Abstracted as a trait so the crate depends on the shape, not a concrete HTTP
/// client, and so the [`SentinelBlockFetcher`] loop can be driven by a mock in
/// tests. The concrete impl (reqwest against the route, decoding the returned
/// `BlockAndOutcome` JSON into [`BlockData`]) lands with the CLI wiring.
#[async_trait]
pub trait BlockRangeClient: Send + Sync {
    /// Fetch the contiguous run of full blocks `[from, to]` (inclusive). The
    /// endpoint caps the run at [`MAX_BLOCK_RANGE`] and stops at the first height
    /// missing on disk, so the result is **ascending and contiguous from `from`**
    /// but may be **shorter** than requested — and empty if `from` itself is not
    /// yet available.
    async fn fetch_block_range(&self, from: u64, to: u64) -> AnyResult<Vec<BlockData>>;
}

/// Concrete [`BlockRangeClient`] over a sentinel's indexer-httpd REST API:
/// `GET /block/full/range?from=&to=`, returning a JSON array of full blocks.
#[derive(Clone)]
pub struct SentinelRangeClient {
    http: reqwest::Client,
    /// `{base}/block/full/range`, joined once at construction.
    range_url: Url,
}

impl SentinelRangeClient {
    /// Build from the sentinel's base URL (e.g. `http://sentinel:8080`).
    pub fn new<U>(base_url: U) -> AnyResult<Self>
    where
        U: IntoUrl,
    {
        Ok(Self {
            http: reqwest::Client::new(),
            range_url: base_url.into_url()?.join("block/full/range")?,
        })
    }
}

#[async_trait]
impl BlockRangeClient for SentinelRangeClient {
    async fn fetch_block_range(&self, from: u64, to: u64) -> AnyResult<Vec<BlockData>> {
        // Build `?from=&to=` via the `url` crate (this reqwest build exposes no
        // `RequestBuilder::query`), then decode the body with serde_json (no
        // `json` feature either). The per-request timeout is applied by the
        // fetcher loop, so this client sets none of its own.
        let mut url = self.range_url.clone();
        url.query_pairs_mut()
            .append_pair("from", &from.to_string())
            .append_pair("to", &to.to_string());

        let bytes = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let blocks = serde_json::from_slice::<Vec<crate::wire::FullBlock>>(&bytes)?
            .into_iter()
            .map(BlockData::from)
            .collect();

        Ok(blocks)
    }
}

/// Tuning for [`SentinelBlockFetcher`].
#[derive(Debug, Clone)]
pub struct SentinelFetcherConfig {
    /// Blocks requested per `/block/full/range` call, clamped to the blocks left
    /// in the gap and capped at [`MAX_BLOCK_RANGE`] (the endpoint's own limit).
    pub range_size: u64,
    /// Per-request timeout before retrying.
    pub timeout: Duration,
    /// Upper bound on blocks fetched-ahead but not yet consumed. The fetcher can
    /// be far faster than the store writer during a backfill; a bounded channel
    /// makes a full buffer block the fetcher (backpressure) instead of letting it
    /// race ahead and balloon RAM. No throughput cost — the consumer is the
    /// bottleneck and the fetcher only needs a small lead. This is a **RAM** knob:
    /// at the measured payloads (median ~20 KB, p90 ~150 KB borsh) 2_000 is
    /// ~40 MB typical / ~300 MB peak.
    pub channel_capacity: usize,
    /// Backoff after a failed, timed-out, or empty response before retrying.
    pub retry_backoff: Duration,
}

impl Default for SentinelFetcherConfig {
    fn default() -> Self {
        Self {
            range_size: MAX_BLOCK_RANGE,
            timeout: Duration::from_secs(30),
            channel_capacity: 2_000,
            retry_backoff: Duration::from_millis(500),
        }
    }
}

/// [`BlockFetcher`] backed by a sentinel's `/block/full/range` endpoint.
///
/// Fills a bounded gap `[from, to]` by pulling contiguous runs of up to
/// `range_size` full blocks per request and streaming the assembled
/// [`BlockData`] in ascending order. Each request starts one past the **last
/// block actually received**, so a short run (the sentinel not yet holding the
/// next height) simply re-requests from there instead of assuming a fixed
/// stride.
///
/// Generic over the client so the crate depends only on the [`BlockRangeClient`]
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
    C: BlockRangeClient + Clone + 'static,
{
    fn spawn(&self, from: u64, to: u64) -> FetchStream {
        let (tx, rx) = mpsc::channel(self.config.channel_capacity);
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

/// Fetch `[from, to]` inclusive in contiguous runs, sending each [`BlockData`] in
/// ascending order through `tx`. Returns when the range is done or the consumer
/// drops the receiver. Transient failures (request error, timeout, or a
/// not-yet-available height) back off and retry — every block in the range
/// exists below the live tip, so a failure is always transient and never
/// surfaced to the consumer.
async fn fetch_range<C>(
    from: u64,
    to: u64,
    tx: mpsc::Sender<BlockData>,
    client: C,
    config: SentinelFetcherConfig,
) where
    C: BlockRangeClient + Clone + 'static,
{
    let range_size = config.range_size.clamp(1, MAX_BLOCK_RANGE);
    let mut next_from = from;

    while next_from <= to {
        // Up to `range_size` blocks, clamped to the blocks left in the gap.
        let batch_to = min(next_from + range_size - 1, to);

        let blocks = match tokio::time::timeout(
            config.timeout,
            client.fetch_block_range(next_from, batch_to),
        )
        .await
        {
            Ok(Ok(blocks)) => blocks,
            Ok(Err(_error)) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_error, next_from, "sentinel range fetch failed, retrying");
                sleep(config.retry_backoff).await;
                continue;
            },
            Err(_timeout) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(next_from, "sentinel range fetch timed out, retrying");
                sleep(config.retry_backoff).await;
                continue;
            },
        };

        // An empty response means `next_from` is not yet on the sentinel — retry
        // from the same height after a backoff.
        let Some(last) = blocks.last().map(|block| block.height()) else {
            sleep(config.retry_backoff).await;
            continue;
        };

        for block in blocks {
            if tx.send(block).await.is_err() {
                // The consumer dropped the `FetchStream`; stop.
                return;
            }
        }

        // Resume one past the last block actually received — a short run just
        // re-requests from the next height.
        next_from = last + 1;
    }
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_primitives::{Block, BlockInfo, BlockOutcome, Hash256, Timestamp},
    };

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

    /// A range client that serves `[from, to]` but caps each response at
    /// `max_per_call` blocks — so it can legitimately return **fewer** than
    /// requested, exercising the "resume from the last received block" loop.
    #[derive(Clone)]
    struct MockRangeClient {
        max_per_call: u64,
    }

    #[async_trait]
    impl BlockRangeClient for MockRangeClient {
        async fn fetch_block_range(&self, from: u64, to: u64) -> AnyResult<Vec<BlockData>> {
            let end = to.min(from + self.max_per_call - 1);
            Ok((from..=end).map(block).collect())
        }
    }

    async fn collect(stream: &mut FetchStream, count: usize) -> Vec<u64> {
        let mut heights = Vec::new();
        for _ in 0..count {
            match stream.recv().await {
                Some(block) => heights.push(block.height()),
                None => break,
            }
        }
        heights
    }

    #[tokio::test]
    async fn fetches_full_range_in_order_then_ends() {
        let fetcher = SentinelBlockFetcher::new(
            MockRangeClient { max_per_call: 20 },
            SentinelFetcherConfig {
                range_size: 20,
                ..SentinelFetcherConfig::default()
            },
        );

        let mut stream = fetcher.spawn(1, 50);
        assert_eq!(collect(&mut stream, 50).await, (1..=50).collect::<Vec<_>>());
        // Stream ends after `to`.
        assert!(stream.recv().await.is_none());
    }

    #[tokio::test]
    async fn resumes_from_last_received_on_short_runs() {
        // The client returns at most 3 blocks per call, well under `range_size`,
        // so the loop must keep advancing via `last + 1`.
        let fetcher =
            SentinelBlockFetcher::new(MockRangeClient { max_per_call: 3 }, SentinelFetcherConfig {
                range_size: 20,
                ..SentinelFetcherConfig::default()
            });

        let mut stream = fetcher.spawn(1, 10);
        assert_eq!(collect(&mut stream, 10).await, (1..=10).collect::<Vec<_>>());
        assert!(stream.recv().await.is_none());
    }
}
