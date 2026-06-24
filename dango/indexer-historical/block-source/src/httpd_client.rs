use {
    anyhow::{Context, bail},
    dango_graphql_ws_client::WsClient,
    dango_indexer_graphql_types::{SubscribeBlock, block, subscribe_block},
    dango_indexer_historical_types::{AnyResult, post_graphql},
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

    /// Open a WebSocket subscription to the `block` channel and return a
    /// stream of block heights as the indexer commits them.
    ///
    /// The underlying subscription emits the full block (transactions,
    /// events) — we project to just `block_height`. Reusing the existing
    /// `SubscribeBlock` query avoids touching `dango-indexer-graphql-types`; if the
    /// payload size becomes a concern, we can add a dedicated lightweight
    /// subscription later.
    pub(crate) async fn subscribe_blocks(&self) -> AnyResult<BoxStream<'static, AnyResult<u64>>> {
        let stream = self
            .ws
            .subscribe::<SubscribeBlock>(subscribe_block::Variables {})
            .await?;

        let mapped = stream.map(|res| -> AnyResult<u64> {
            let response = res?;

            if let Some(errors) = response.errors
                && !errors.is_empty()
            {
                bail!("subscription returned errors: {errors:?}");
            }

            let data = response.data.context("subscription returned no data")?;
            Ok(data.block.block_height as u64)
        });

        Ok(Box::pin(mapped))
    }
}
