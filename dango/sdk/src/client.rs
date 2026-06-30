use {
    anyhow::{anyhow, bail, ensure},
    async_trait::async_trait,
    dango_backtrace::BacktracedError,
    dango_indexer_graphql_types::{
        PageInfo, Variables, accounts, blocks, broadcast_tx_sync, events, messages, query_app,
        search_tx, simulate, transactions, transfers,
    },
    dango_primitives::{
        Addr, Block, BlockClient, BlockOutcome, BroadcastClient, BroadcastTxOutcome, GenericResult,
        Hash256, Inner, Json, JsonDeExt, JsonSerExt, NonEmpty, Query, QueryClient, QueryResponse,
        SearchTxClient, SearchTxOutcome, Tx, TxOutcome, UnsignedTx,
    },
    futures::{SinkExt, Stream, StreamExt, channel::mpsc},
    graphql_client::{GraphQLQuery, Response},
    reqwest::IntoUrl,
    serde::{Deserialize, Serialize},
    std::{fmt::Debug, str::FromStr, time::Duration},
    tokio_tungstenite::{connect_async, tungstenite::Message},
    url::Url,
};

#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
    url: Url,
}

impl HttpClient {
    pub fn new<U>(url: U) -> anyhow::Result<Self>
    where
        U: IntoUrl,
    {
        Ok(Self {
            inner: reqwest::Client::new(),
            url: url.into_url()?,
        })
    }

    async fn get(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        error_for_status(self.inner.get(self.url.join(path)?).send().await?).await
    }

    async fn post_graphql<V>(
        &self,
        variables: V,
    ) -> anyhow::Result<<V::Query as GraphQLQuery>::ResponseData>
    where
        V: Variables + Serialize + Debug,
        <<V as dango_indexer_graphql_types::Variables>::Query as graphql_client::GraphQLQuery>::ResponseData:
            Debug,
    {
        let query = V::Query::build_query(variables);
        let response = error_for_status(
            self.inner
                .post(self.url.join("graphql")?)
                .json(&query)
                .send()
                .await?,
        )
        .await?;

        #[cfg(feature = "tracing")]
        {
            tracing::debug!("GraphQL request: {query:#?}");
            tracing::debug!("GraphQL response: {response:#?}");
        }

        let body: Response<<V::Query as GraphQLQuery>::ResponseData> = response.json().await?;

        match body.data {
            Some(data) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("GraphQL body response: {data:#?}");

                Ok(data)
            },
            None => bail!("no data returned from query: errors: {:?}", body.errors),
        }
    }

    /// Paginate through all results of a GraphQL query using cursor-based pagination.
    ///
    /// This method handles the pagination loop, collecting all items across pages.
    /// It supports both forward pagination (using `first`) and backward pagination
    /// (using `last`).
    ///
    /// ## Arguments
    ///
    /// - `first` - Number of items to fetch per page when paginating forward (use with `None` for `last`)
    /// - `last` - Number of items to fetch per page when paginating backward (use with `None` for `first`)
    /// - `build_variables` - Closure that builds the query variables given pagination cursors
    /// - `extract_page` - Closure that extracts the nodes and page info from the response data
    ///
    /// ## Example
    ///
    /// ```ignore
    /// let all_accounts = client
    ///     .paginate_all(
    ///         Some(10), // fetch 10 per page, forward pagination
    ///         None,
    ///         |after, before, first, last| accounts::Variables {
    ///             after,
    ///             before,
    ///             first: first.map(|f| f as i64),
    ///             last: last.map(|l| l as i64),
    ///             ..Default::default()
    ///         },
    ///         |data| {
    ///             let page_info = PageInfo {
    ///                 start_cursor: data.accounts.page_info.start_cursor,
    ///                 end_cursor: data.accounts.page_info.end_cursor,
    ///                 has_next_page: data.accounts.page_info.has_next_page,
    ///                 has_previous_page: data.accounts.page_info.has_previous_page,
    ///             };
    ///             (data.accounts.nodes, page_info)
    ///         },
    ///     )
    ///     .await?;
    /// ```
    pub async fn paginate_all<V, N, BuildVariables, ExtractPage>(
        &self,
        first: Option<i64>,
        last: Option<i64>,
        build_variables: BuildVariables,
        extract_page: ExtractPage,
    ) -> anyhow::Result<Vec<N>>
    where
        V: Variables + Serialize + Debug,
        <V::Query as GraphQLQuery>::ResponseData: Debug,
        BuildVariables: Fn(Option<String>, Option<String>, Option<i64>, Option<i64>) -> V,
        ExtractPage: Fn(<V::Query as GraphQLQuery>::ResponseData) -> (Vec<N>, PageInfo),
    {
        let mut all_items = vec![];
        let mut after = None;
        let mut before = None;

        loop {
            let variables = build_variables(after.clone(), before.clone(), first, last);
            let data = self.post_graphql(variables).await?;
            let (nodes, page_info) = extract_page(data);

            match (first, last) {
                (Some(_), None) => {
                    // Forward pagination
                    all_items.extend(nodes);
                    if !page_info.has_next_page {
                        break;
                    }
                    after = page_info.end_cursor;
                    if after.is_none() {
                        break;
                    }
                },
                (None, Some(_)) => {
                    // Backward pagination - items come in reverse order
                    all_items.extend(nodes.into_iter().rev());
                    if !page_info.has_previous_page {
                        break;
                    }
                    before = page_info.start_cursor;
                    if before.is_none() {
                        break;
                    }
                },
                _ => {
                    // Invalid: must specify exactly one of first or last
                    bail!("paginate_all requires exactly one of `first` or `last` to be Some");
                },
            }
        }

        Ok(all_items)
    }

    /// Subscribe to perps-exchange events over the WebSocket endpoint
    /// (`GET /ws`, `perpsEvents` channel) — the transport replacement for the
    /// `perps_events` GraphQL subscription. Yields one [`PerpsEventsBatch`] per
    /// block that has at least one matching event.
    ///
    /// The five filters are sets that AND together; `None` (or an empty list)
    /// does not filter on that field. `since_block_height` replays the retained
    /// in-memory window from that height (inclusive) before the live tail; a
    /// height that predates the window fails this call with an error (the
    /// server's `resync` reply). The subscription is also subject to the
    /// server's limit (`tooManyRequests`).
    pub async fn subscribe_perps_events(
        &self,
        since_block_height: Option<u64>,
        event_types: Option<Vec<String>>,
        pair_ids: Option<Vec<String>>,
        users: Option<Vec<String>>,
        order_ids: Option<Vec<String>>,
        client_order_ids: Option<Vec<String>>,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<PerpsEventsBatch>> + Send> {
        // Derive the `/ws` URL from the HTTP base.
        let mut ws_url = self.url.join("ws")?;
        match ws_url.scheme() {
            "http" => ws_url
                .set_scheme("ws")
                .map_err(|_| anyhow!("failed to set ws scheme"))?,
            "https" => ws_url
                .set_scheme("wss")
                .map_err(|_| anyhow!("failed to set wss scheme"))?,
            "ws" | "wss" => {},
            scheme => bail!("invalid URL scheme: {scheme}"),
        }

        // Build the subscribe message. Absent filters are omitted (match-all);
        // present ones are sent as JSON arrays.
        let subscribe = {
            let mut subscription = serde_json::Map::new();
            subscription.insert("type".into(), "perpsEvents".into());

            if let Some(since) = since_block_height {
                subscription.insert("since".into(), since.into());
            }

            for (key, values) in [
                ("eventTypes", event_types),
                ("pairIds", pair_ids),
                ("users", users),
                ("orderIds", order_ids),
                ("clientOrderIds", client_order_ids),
            ] {
                if let Some(values) = values {
                    subscription.insert(key.into(), values.into());
                }
            }

            serde_json::json!({
                "method": "subscribe",
                "id": 1,
                "subscription": serde_json::Value::Object(subscription),
            })
        };

        let (ws, _response) = connect_async(ws_url.as_str())
            .await
            .map_err(|err| anyhow!("WebSocket connection failed: {err}"))?;

        let (mut sink, mut stream) = ws.split();
        sink.send(Message::text(subscribe.to_string()))
            .await
            .map_err(|err| anyhow!("failed to send subscribe: {err}"))?;

        // Await the acknowledgement, mapping a `resync` / `tooManyRequests`
        // error frame to a connect-time error (no data frame precedes the ack).
        loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => match parse_server_frame(&text)? {
                    ServerFrame::Ack => break,
                    ServerFrame::Error { code, message } => bail!("{code}: {message}"),
                    ServerFrame::Data(_) | ServerFrame::Other => {},
                },
                Some(Ok(Message::Ping(data))) => {
                    let _ = sink.send(Message::Pong(data)).await;
                },
                Some(Ok(Message::Close(frame))) => {
                    bail!("WebSocket closed before acknowledgement: {frame:?}")
                },
                Some(Ok(_)) => {},
                Some(Err(err)) => bail!("WebSocket error before acknowledgement: {err}"),
                None => bail!("WebSocket closed before acknowledgement"),
            }
        }

        // Drive the socket on a background task: forward batches to the stream,
        // answer control pings, and send an app-level ping on a fixed schedule
        // so an idle subscription is not reaped by the server's idle timeout.
        let (tx, rx) = mpsc::unbounded::<anyhow::Result<PerpsEventsBatch>>();
        tokio::spawn(async move {
            let mut keepalive = tokio::time::interval(Duration::from_secs(20));
            keepalive.tick().await; // The first tick is immediate; skip it.
            let ping = serde_json::json!({"method": "ping"}).to_string();

            loop {
                tokio::select! {
                    _ = keepalive.tick() => {
                        if sink.send(Message::text(ping.clone())).await.is_err() {
                            break;
                        }
                    },
                    message = stream.next() => match message {
                        Some(Ok(Message::Text(text))) => match parse_server_frame(&text) {
                            Ok(ServerFrame::Data(batch)) => {
                                if tx.unbounded_send(Ok(batch)).is_err() {
                                    break;
                                }
                            },
                            Ok(ServerFrame::Error { code, message }) => {
                                let _ = tx.unbounded_send(Err(anyhow!("{code}: {message}")));
                                break;
                            },
                            Ok(ServerFrame::Ack | ServerFrame::Other) => {},
                            Err(err) => {
                                let _ = tx.unbounded_send(Err(err));
                                break;
                            },
                        },
                        Some(Ok(Message::Ping(data))) => {
                            if sink.send(Message::Pong(data)).await.is_err() {
                                break;
                            }
                        },
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                        Some(Ok(_)) => {},
                    },
                }
            }

            let _ = sink.close().await;
        });

        Ok(rx)
    }
}

/// Macro to generate pagination methods for GraphQL queries.
///
/// This macro generates a `paginate_X` method on `HttpClient` that handles
/// cursor-based pagination for a specific query type.
///
/// # Arguments
///
/// * `$method_name` - The name of the generated method (e.g., `paginate_accounts`)
/// * `$module` - The module containing the query types (e.g., `accounts`)
/// * `$field` - The response field name (e.g., `accounts`)
/// * `$node_type` - The node type returned by the query (e.g., `AccountsAccountsNodes`)
macro_rules! impl_paginate_method {
    ($method_name:ident, $module:ident, $field:ident, $node_type:ident) => {
        impl HttpClient {
            /// Paginate through all results, returning all nodes.
            ///
            /// # Arguments
            ///
            /// * `page_size` - Number of items to fetch per page
            /// * `variables` - Query variables (pagination fields will be overwritten)
            ///
            /// # Example
            ///
            /// ```ignore
            #[doc = concat!("let all_items = client.", stringify!($method_name), "(")]
            ///     10,
            #[doc = concat!("    ", stringify!($module), "::Variables {")]
            ///         sort_by: Some(SortBy::DESC),
            ///         ..Default::default()
            ///     },
            /// ).await?;
            /// ```
            pub async fn $method_name(
                &self,
                page_size: i64,
                mut variables: $module::Variables,
            ) -> anyhow::Result<Vec<$module::$node_type>> {
                let mut all_items = vec![];
                let mut after: Option<String> = None;

                loop {
                    variables.after = after.clone();
                    variables.before = None;
                    variables.first = Some(page_size);
                    variables.last = None;

                    let data = self.post_graphql(variables.clone()).await?;
                    let connection = data.$field;

                    all_items.extend(connection.nodes);

                    if !connection.page_info.has_next_page {
                        break;
                    }
                    after = connection.page_info.end_cursor;
                    if after.is_none() {
                        break;
                    }
                }

                Ok(all_items)
            }
        }
    };
}

// Generate pagination methods for all paginated query types
impl_paginate_method!(paginate_accounts, accounts, accounts, AccountsAccountsNodes);
impl_paginate_method!(
    paginate_transfers,
    transfers,
    transfers,
    TransfersTransfersNodes
);
impl_paginate_method!(
    paginate_transactions,
    transactions,
    transactions,
    TransactionsTransactionsNodes
);
impl_paginate_method!(paginate_blocks, blocks, blocks, BlocksBlocksNodes);
impl_paginate_method!(paginate_events, events, events, EventsEventsNodes);
impl_paginate_method!(paginate_messages, messages, messages, MessagesMessagesNodes);

#[async_trait]
impl QueryClient for HttpClient {
    type Error = anyhow::Error;
    type Proof = dango_primitives::Proof;

    async fn query_app(&self, query: Query) -> Result<QueryResponse, Self::Error> {
        let response = self
            .post_graphql(query_app::Variables {
                request: query.to_json_value()?.into_inner(),
                height: None,
            })
            .await?;

        // TODO
        Ok(serde_json::from_value(response.query_app)?)
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        let response = self
            .post_graphql(simulate::Variables {
                tx: tx.to_json_value()?.into_inner(),
            })
            .await?;

        Ok(serde_json::from_value(response.simulate)?)
    }
}

#[async_trait]
impl BlockClient for HttpClient {
    type Error = anyhow::Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error> {
        let path = match height {
            Some(height) => format!("block/info/{height}"),
            None => "block/info".to_string(),
        };

        Ok(self.get(&path).await?.json().await?)
    }

    async fn query_block_outcome(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error> {
        let path = match height {
            Some(height) => format!("block/result/{height}"),
            None => "block/result".to_string(),
        };

        Ok(self.get(&path).await?.json().await?)
    }
}

#[async_trait]
impl BroadcastClient for HttpClient {
    type Error = anyhow::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        let response = self
            .post_graphql(broadcast_tx_sync::Variables {
                tx: tx.to_json_value()?.into_inner(),
            })
            .await?
            .broadcast_tx_sync;

        Ok(serde_json::from_value(response)?)
    }
}

#[async_trait]
impl SearchTxClient for HttpClient {
    type Error = anyhow::Error;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error> {
        let mut response = self
            .post_graphql(search_tx::Variables {
                hash: hash.to_string(),
            })
            .await?
            .transactions
            .nodes;

        let res = response.pop().ok_or(anyhow!("tx not found: {hash}"))?;

        ensure!(response.is_empty(), "multiple txs found for hash: {hash}");

        let msgs = res
            .messages
            .iter()
            .map(|m| Json::from_inner(m.data.clone()).deserialize_json())
            .collect::<Result<Vec<_>, _>>()?;

        let tx = Tx {
            sender: Addr::from_str(&res.sender)?,
            gas_limit: res.gas_wanted as u64,
            msgs: NonEmpty::new(msgs)?,
            data: Json::from_inner(res.data.clone()),
            credential: Json::from_inner(res.credential.clone()),
        };

        Ok(SearchTxOutcome {
            hash,
            height: res.block_height as u64,
            index: res.transaction_idx as u32,
            tx,
            outcome: TxOutcome {
                gas_limit: res.gas_wanted as u64,
                gas_used: res.gas_used as u64,
                result: if res.has_succeeded {
                    GenericResult::Ok(())
                } else {
                    GenericResult::Err(
                        res.error_message
                            .map(|e| e.deserialize_json())
                            .transpose()?
                            .unwrap_or_else(|| {
                                BacktracedError::new_without_bt("error not found!".to_string())
                            }),
                    )
                },
                events: res
                    .nested_events
                    .clone()
                    .ok_or(anyhow!("no nested events"))?
                    .deserialize_json()?,
            },
        })
    }
}

// ---- perps events WebSocket feed ----

/// One block's matching perps-contract events, as delivered by the `/ws`
/// `perpsEvents` channel. Mirrors the `perps_events` payload shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpsEventsBatch {
    pub block_height: u64,
    /// Block timestamp, RFC 3339.
    pub created_at: String,
    pub events: Vec<PerpsEvent>,
}

/// A single perps-contract event within a [`PerpsEventsBatch`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerpsEvent {
    pub idx: u32,
    pub event_type: String,
    pub user: Option<String>,
    pub pair_id: Option<String>,
    pub order_id: Option<String>,
    pub client_order_id: Option<String>,
    pub data: serde_json::Value,
}

/// A classified server frame from the `/ws` endpoint.
enum ServerFrame {
    /// `subscriptionResponse` acknowledgement.
    Ack,

    /// A `perpsEvents` data frame.
    Data(PerpsEventsBatch),

    /// An `error` frame (e.g. `resync`, `tooManyRequests`).
    Error { code: String, message: String },

    /// Anything else (e.g. `pong`).
    Other,
}

fn parse_server_frame(text: &str) -> anyhow::Result<ServerFrame> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    // An error is co-located on the operation's own channel as an `error`-keyed
    // frame; a connection-level error uses the dedicated `error` channel. Check
    // for `error` first, regardless of channel, so a terminal `resync` on the
    // `perpsEvents` channel is surfaced as an error rather than parsed as data.
    if let Some(error) = value.get("error") {
        return Ok(ServerFrame::Error {
            code: error
                .get("code")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("error")
                .to_string(),
            message: error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }

    match value.get("channel").and_then(serde_json::Value::as_str) {
        Some("subscriptionResponse") => Ok(ServerFrame::Ack),
        Some("perpsEvents") => {
            let data = value.get("data").cloned().unwrap_or_default();
            Ok(ServerFrame::Data(serde_json::from_value(data)?))
        },
        _ => Ok(ServerFrame::Other),
    }
}

async fn error_for_status(response: reqwest::Response) -> anyhow::Result<reqwest::Response> {
    if let Err(e) = response.error_for_status_ref() {
        bail!("{}: {}", e, response.text().await?)
    } else {
        Ok(response)
    }
}
