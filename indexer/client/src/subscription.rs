use {
    crate::Variables,
    anyhow::bail,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc},
    graphql_client::{GraphQLQuery, Response},
    serde::{Deserialize, Serialize},
    std::{pin::Pin, time::Duration},
    tokio_tungstenite::{
        connect_async,
        tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
    },
    url::Url,
};

/// Must stay below the server's `GraphQLSubscription::keepalive_timeout` (30s)
/// or idle subscriptions are dropped with close code 3008.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

const SUBSCRIPTION_ID: &str = "1";
const CHANNEL_CAPACITY: usize = 32;

/// Errors surfaced on a [`SubscriptionStream`].
#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("WebSocket connection closed: {0}")]
    Closed(String),
    #[error("WebSocket transport error: {0}")]
    Transport(String),
    #[error("subscription returned error: {0}")]
    Subscription(serde_json::Value),
    #[error("failed to decode message: {0}")]
    Decode(String),
}

/// A WebSocket client for GraphQL subscriptions.
///
/// Implements the `graphql-transport-ws` protocol directly on top of
/// [`tokio_tungstenite`] so that `ping` messages are sent on a fixed schedule
/// regardless of inbound traffic — required because `async-graphql` only
/// resets its `keepalive_timeout` when it receives a client protocol message.
#[derive(Debug, Clone)]
pub struct WsClient {
    url: Url,
}

/// Type alias for the subscription stream returned by the client.
pub type SubscriptionStream<T> = Pin<Box<dyn Stream<Item = Result<Response<T>, WsError>> + Send>>;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage<'a> {
    ConnectionInit,
    Subscribe {
        id: &'a str,
        payload: serde_json::Value,
    },
    Ping,
    Pong,
    Complete {
        id: &'a str,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    ConnectionAck,
    Ping,
    Pong,
    Next {
        id: String,
        payload: serde_json::Value,
    },
    Error {
        id: String,
        payload: serde_json::Value,
    },
    Complete {
        id: String,
    },
}

impl WsClient {
    /// Create a new WebSocket client.
    ///
    /// The URL should be a WebSocket URL (ws:// or wss://) pointing to the GraphQL endpoint.
    pub fn new(url: impl Into<String>) -> Result<Self, anyhow::Error> {
        let url_str = url.into();
        let url = Url::parse(&url_str)?;

        match url.scheme() {
            "ws" | "wss" => {},
            scheme => bail!("Invalid URL scheme: {scheme}. Expected ws:// or wss://"),
        }

        Ok(Self { url })
    }

    /// Create a new WebSocket client from an HTTP URL.
    ///
    /// Converts http:// to ws:// and https:// to wss://.
    pub fn from_http_url(url: impl Into<String>) -> Result<Self, anyhow::Error> {
        let url_str = url.into();
        let mut url = Url::parse(&url_str)?;

        match url.scheme() {
            "http" => url
                .set_scheme("ws")
                .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?,
            "https" => url
                .set_scheme("wss")
                .map_err(|_| anyhow::anyhow!("Failed to set scheme"))?,
            "ws" | "wss" => {},
            scheme => {
                bail!("Invalid URL scheme: {scheme}. Expected http://, https://, ws://, or wss://")
            },
        }

        Ok(Self { url })
    }

    /// Subscribe to a GraphQL subscription.
    ///
    /// Returns a stream of subscription responses wrapped in `graphql_client::Response`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use futures::StreamExt;
    /// use indexer_client::{WsClient, SubscribeTrades, subscribe_trades};
    ///
    /// let client = WsClient::new("ws://localhost:8080/graphql")?;
    ///
    /// let variables = subscribe_trades::Variables {
    ///     base_denom: "dango".to_string(),
    ///     quote_denom: "bridge/usdc".to_string(),
    /// };
    ///
    /// let mut stream = client.subscribe::<SubscribeTrades>(variables).await?;
    ///
    /// while let Some(response) = stream.next().await {
    ///     match response {
    ///         Ok(resp) => {
    ///             if let Some(data) = resp.data {
    ///                 println!("{:?}", data);
    ///             }
    ///         }
    ///         Err(e) => eprintln!("Error: {e}"),
    ///     }
    /// }
    /// ```
    pub async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<SubscriptionStream<Q::ResponseData>, anyhow::Error>
    where
        Q: GraphQLQuery + Unpin + Send + Sync + 'static,
        Q::Variables: Unpin + Send + Sync + 'static,
        Q::ResponseData: Unpin + Send + Sync + 'static,
    {
        let mut request = self.url.as_str().into_client_request()?;

        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_static("graphql-transport-ws"),
        );

        #[cfg(feature = "tracing")]
        tracing::debug!("Connecting to WebSocket: {}", self.url);

        let (ws, _response) = connect_async(request)
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {e}"))?;

        let (mut sink, mut stream) = ws.split();

        sink.send(Message::text(serde_json::to_string(
            &ClientMessage::ConnectionInit,
        )?))
        .await
        .map_err(|e| anyhow::anyhow!("failed to send connection_init: {e}"))?;

        // Wait for `connection_ack`, responding to any `ping` in between.
        loop {
            match stream.next().await {
                Some(Ok(Message::Text(txt))) => {
                    match serde_json::from_str::<ServerMessage>(&txt)? {
                        ServerMessage::ConnectionAck => break,
                        ServerMessage::Ping => {
                            sink.send(Message::text(serde_json::to_string(&ClientMessage::Pong)?))
                                .await?;
                        },
                        _ => bail!("unexpected message before connection_ack: {txt}"),
                    }
                },
                Some(Ok(Message::Ping(data))) => {
                    sink.send(Message::Pong(data)).await?;
                },
                Some(Ok(_)) => {},
                Some(Err(e)) => bail!("WebSocket error before connection_ack: {e}"),
                None => bail!("WebSocket closed before connection_ack"),
            }
        }

        let body = Q::build_query(variables);
        let payload = serde_json::json!({
            "query": body.query,
            "variables": body.variables,
            "operationName": body.operation_name,
        });

        sink.send(Message::text(serde_json::to_string(
            &ClientMessage::Subscribe {
                id: SUBSCRIPTION_ID,
                payload,
            },
        )?))
        .await
        .map_err(|e| anyhow::anyhow!("failed to send subscribe: {e}"))?;

        #[cfg(feature = "tracing")]
        tracing::debug!("Subscription established");

        let (tx, rx) =
            mpsc::channel::<Result<Response<Q::ResponseData>, WsError>>(CHANNEL_CAPACITY);

        tokio::spawn(run_subscription::<Q>(sink, stream, tx));

        Ok(Box::pin(rx))
    }
}

async fn run_subscription<Q>(
    mut sink: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    mut stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    mut tx: mpsc::Sender<Result<Response<Q::ResponseData>, WsError>>,
) where
    Q: GraphQLQuery + Unpin + Send + Sync + 'static,
    Q::ResponseData: Unpin + Send + Sync + 'static,
{
    let mut ping_interval = tokio::time::interval(KEEP_ALIVE_INTERVAL);
    // First tick fires immediately; skip it so we don't ping right after subscribe.
    ping_interval.tick().await;

    let ping_payload = serde_json::to_string(&ClientMessage::Ping)
        .expect("serializing unit enum variant never fails");
    let pong_payload = serde_json::to_string(&ClientMessage::Pong)
        .expect("serializing unit enum variant never fails");
    let complete_payload = serde_json::to_string(&ClientMessage::Complete {
        id: SUBSCRIPTION_ID,
    })
    .expect("serializing Complete never fails");

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if sink.send(Message::text(ping_payload.clone())).await.is_err() {
                    break;
                }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        let decoded: ServerMessage = match serde_json::from_str(&txt) {
                            Ok(m) => m,
                            Err(e) => {
                                let _ = tx.send(Err(WsError::Decode(e.to_string()))).await;
                                break;
                            },
                        };

                        match decoded {
                            ServerMessage::Next { id, payload } => {
                                if id != SUBSCRIPTION_ID {
                                    continue;
                                }
                                match serde_json::from_value::<Response<Q::ResponseData>>(payload) {
                                    Ok(resp) => {
                                        if tx.send(Ok(resp)).await.is_err() {
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        let _ =
                                            tx.send(Err(WsError::Decode(e.to_string()))).await;
                                        break;
                                    },
                                }
                            },
                            ServerMessage::Error { id, payload } => {
                                if id != SUBSCRIPTION_ID {
                                    continue;
                                }
                                let _ = tx.send(Err(WsError::Subscription(payload))).await;
                                break;
                            },
                            ServerMessage::Complete { id } => {
                                if id == SUBSCRIPTION_ID {
                                    break;
                                }
                            },
                            ServerMessage::Ping => {
                                if sink
                                    .send(Message::text(pong_payload.clone()))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            },
                            ServerMessage::Pong | ServerMessage::ConnectionAck => {},
                        }
                    },
                    Some(Ok(Message::Ping(data))) => {
                        if sink.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    },
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| format!("{} (code {})", f.reason, u16::from(f.code)))
                            .unwrap_or_else(|| "no close frame".to_string());
                        let _ = tx.send(Err(WsError::Closed(reason))).await;
                        break;
                    },
                    Some(Ok(_)) => {},
                    Some(Err(e)) => {
                        let _ = tx.send(Err(WsError::Transport(e.to_string()))).await;
                        break;
                    },
                    None => break,
                }
            }
        }
    }

    let _ = sink.send(Message::text(complete_payload)).await;
    let _ = sink.close().await;
}

/// Helper trait for subscription variables with an associated subscription type.
pub trait SubscriptionVariables: Variables {
    /// Subscribe using these variables.
    fn subscribe(
        self,
        client: &WsClient,
    ) -> impl std::future::Future<
        Output = Result<
            SubscriptionStream<<<Self as Variables>::Query as GraphQLQuery>::ResponseData>,
            anyhow::Error,
        >,
    > + Send
    where
        Self: Sized + Unpin + Send + Sync + 'static,
        <Self as Variables>::Query: Unpin + Send + Sync + 'static,
        <<Self as Variables>::Query as GraphQLQuery>::ResponseData: Unpin + Send + Sync + 'static,
    {
        client.subscribe::<Self::Query>(self)
    }
}

// Implement SubscriptionVariables for all subscription variable types
impl SubscriptionVariables for crate::subscribe_block::Variables {}
impl SubscriptionVariables for crate::subscribe_accounts::Variables {}
impl SubscriptionVariables for crate::subscribe_transfers::Variables {}
impl SubscriptionVariables for crate::subscribe_transactions::Variables {}
impl SubscriptionVariables for crate::subscribe_messages::Variables {}
impl SubscriptionVariables for crate::subscribe_events::Variables {}
impl SubscriptionVariables for crate::subscribe_event_by_addresses::Variables {}
impl SubscriptionVariables for crate::subscribe_candles::Variables {}
impl SubscriptionVariables for crate::subscribe_perps_candles::Variables {}
impl SubscriptionVariables for crate::subscribe_trades::Variables {}
impl SubscriptionVariables for crate::subscribe_query_app::Variables {}
impl SubscriptionVariables for crate::subscribe_query_store::Variables {}
impl SubscriptionVariables for crate::subscribe_query_status::Variables {}
