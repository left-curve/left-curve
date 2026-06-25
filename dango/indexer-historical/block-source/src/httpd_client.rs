use {
    crate::wire::FullBlock,
    anyhow::{Context, bail},
    dango_graphql_ws_client::WsClient,
    dango_indexer_graphql_types::{SubscribeFullBlock, block, subscribe_full_block},
    dango_indexer_historical_types::{AnyResult, BlockData, post_graphql},
    futures::{StreamExt, stream::BoxStream},
    reqwest::IntoUrl,
    std::time::Duration,
    url::Url,
};

/// Per-request timeout for HTTP calls to `dango-httpd`. Picked to cover slow
/// boot-time queries without letting a stuck connection hang the indexer.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Thin GraphQL client against a dango node's `httpd` — HTTP for queries,
/// WebSocket for subscriptions.
///
/// Shared by both block sources, which speak the same `block` subscription
/// protocol — only the target differs: [`LocalBlockSource`] points it at the
/// co-located in-process `dango-httpd`, while the remote sentinel subscriber
/// points it at a remote sentinel node. It supplies the live block-height
/// notifications and the boot-time frontier query; a detached remote source
/// fetches the block payloads separately over RPC (it has no local disk).
///
/// [`LocalBlockSource`]: crate::LocalBlockSource
#[derive(Debug, Clone)]
pub(crate) struct HttpdClient {
    inner: reqwest::Client,
    graphql_url: Url,
    ws: WsClient,
}

impl HttpdClient {
    /// Construct from the dango-httpd base URL (e.g. `http://localhost:8080`).
    /// The `/graphql` suffix is joined internally; the WebSocket URL is derived
    /// by switching the scheme (`http` → `ws`, `https` → `wss`).
    pub(crate) fn new<U>(base_url: U) -> AnyResult<Self>
    where
        U: IntoUrl,
    {
        let graphql_url = base_url.into_url()?.join("graphql")?;
        let ws = WsClient::from_http_url(graphql_url.as_str())?;
        let inner = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?;

        Ok(Self {
            inner,
            graphql_url,
            ws,
        })
    }

    /// Returns the latest indexed block height, or `None` if the indexer is
    /// at genesis (no blocks yet).
    ///
    /// Reuses the existing `block(height: None)` query from
    /// `dango-indexer-graphql-types`: it carries a heavier payload than strictly
    /// needed (transactions, events), but is called once at boot — the
    /// overhead is acceptable.
    pub(crate) async fn latest_block_height(&self) -> AnyResult<Option<u64>> {
        let data = post_graphql(&self.inner, &self.graphql_url, block::Variables {
            height: None,
        })
        .await?;

        Ok(data.block.map(|b| b.block_height as u64))
    }

    /// Open a WebSocket subscription to the `full_block` channel and return a
    /// stream of fully-assembled [`BlockData`]. With `since`, the feed replays
    /// from that height and then streams the live tail; without it, it streams
    /// only blocks newer than the current tip. Each item is the sentinel's
    /// `BlockAndOutcome` as a JSON scalar, decoded here.
    ///
    /// This is the **shared live path**: both block sources call it — the local
    /// one against the in-process `dango-httpd`, the remote one against a
    /// sentinel — so the live-tail logic lives in exactly one place.
    pub(crate) async fn subscribe_full_blocks(
        &self,
        since: Option<u64>,
    ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>> {
        let stream = self
            .ws
            .subscribe::<SubscribeFullBlock>(subscribe_full_block::Variables {
                since_block_height: since.map(|height| height as i64),
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
            // `full_block` is the JSON scalar — the sentinel's `BlockAndOutcome`.
            Ok(serde_json::from_value::<FullBlock>(data.full_block)?.into())
        });

        Ok(Box::pin(mapped))
    }
}
