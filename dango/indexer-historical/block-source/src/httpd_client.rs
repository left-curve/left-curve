use {
    crate::remote::BlockRangeClient,
    anyhow::{Context, bail},
    async_trait::async_trait,
    dango_graphql_ws_client::WsClient,
    dango_indexer_graphql_types::{SubscribeFullBlock, subscribe_full_block},
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::{StreamExt, stream::BoxStream},
    reqwest::IntoUrl,
    std::time::Duration,
    url::Url,
};

/// Per-request timeout for the REST range calls. The subscription is long-lived
/// and sets none; the fetcher loop additionally wraps each range call in its own
/// timeout.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Thin client against a dango node's `httpd` — WebSocket for the `full_block`
/// subscription, HTTP for the `/block/full/range` backfill endpoint.
///
/// Used directly by both block sources (no trait): there is one node-backed
/// implementation. The local source points it at the in-process `dango-httpd`,
/// the remote source at a sentinel — only the base URL differs. It is the live
/// path (`subscribe_full_blocks`) and, via [`BlockRangeClient`], the backfill
/// path the [`SentinelBlockFetcher`] drives.
///
/// [`SentinelBlockFetcher`]: crate::SentinelBlockFetcher
#[derive(Debug, Clone)]
pub struct HttpdClient {
    inner: reqwest::Client,
    /// `{base}/block/full/range`, joined once at construction.
    range_url: Url,
    ws: WsClient,
}

impl HttpdClient {
    /// Construct from the node's base URL (e.g. `http://localhost:8080`). The
    /// `/graphql` and `/block/full/range` paths are joined internally; the
    /// WebSocket URL is derived from the GraphQL one.
    pub fn new<U>(base_url: U) -> AnyResult<Self>
    where
        U: IntoUrl,
    {
        let base = base_url.into_url()?;
        let graphql_url = base.join("graphql")?;
        let range_url = base.join("block/full/range")?;
        let ws = WsClient::from_http_url(graphql_url.as_str())?;
        let inner = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?;

        Ok(Self {
            inner,
            range_url,
            ws,
        })
    }

    /// Open a WebSocket subscription to the `full_block` channel and return a
    /// stream of fully-assembled [`BlockData`], **always starting at the live
    /// tip** (only blocks newer than the current tip). Each item is the node's
    /// `FullBlock` as a JSON scalar, decoded here.
    ///
    /// There is deliberately **no `since` parameter**. The node serves this feed
    /// from a small in-memory ring (~100 blocks), so a `since` below that window
    /// fails the subscription with a "resync required" error — which is exactly
    /// where the resume point sits during a backfill or after any non-trivial
    /// downtime. Resuming at the tip and backfilling the gap below it by other
    /// means — the remote source's healer via `/block/full/range`, the local
    /// source's on-disk `get` — is the only reconnect strategy that cannot wedge.
    /// See the callers in `remote::drain_live` and `LocalBlockSource::run`.
    ///
    /// The **shared live path**: both block sources call it — the local one
    /// against the in-process `dango-httpd`, the remote one against a sentinel.
    pub(crate) async fn subscribe_full_blocks(
        &self,
    ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>> {
        let stream = self
            .ws
            .subscribe::<SubscribeFullBlock>(subscribe_full_block::Variables {
                // Always at the tip — see the doc comment for why there is no
                // `since`.
                since_block_height: None,
            })
            .await?;

        let mapped = stream.map(|res| -> AnyResult<BlockData> {
            let response = res?;

            if let Some(errors) = response.errors
                && !errors.is_empty()
            {
                bail!("full_block subscription returned errors: {errors:?}");
            }

            let data = response
                .data
                .context("full_block subscription returned no data")?;
            // `full_block` is the JSON scalar — the sentinel's `FullBlock`, which
            // `BlockData` (an alias of it) deserializes directly.
            Ok(serde_json::from_value::<BlockData>(data.full_block)?)
        });

        Ok(Box::pin(mapped))
    }
}

#[async_trait]
impl BlockRangeClient for HttpdClient {
    async fn fetch_block_range(&self, from: u64, to: u64) -> AnyResult<Vec<BlockData>> {
        // GET /block/full/range?from=&to=. The query string is built with the
        // `url` crate (this reqwest build has no RequestBuilder::query) and the
        // body decoded with serde_json (no `json` feature). `BlockData` decodes
        // the `{ block, outcome }` items directly.
        let mut url = self.range_url.clone();
        url.query_pairs_mut()
            .append_pair("from", &from.to_string())
            .append_pair("to", &to.to_string());

        let bytes = self
            .inner
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(serde_json::from_slice::<Vec<BlockData>>(&bytes)?)
    }
}
