use {
    crate::Variables,
    anyhow::bail,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc,
        stream::{SplitSink, SplitStream},
    },
    graphql_client::{GraphQLQuery, Response},
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::{
        collections::HashMap,
        marker::PhantomData,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        task::{Context, Poll},
        time::Duration,
    },
    tokio_tungstenite::{
        MaybeTlsStream, WebSocketStream, connect_async,
        tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
    },
    url::Url,
};

/// Must stay below the server's `GraphQLSubscription::keepalive_timeout` (30s)
/// or idle subscriptions are dropped with close code 3008.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type ResponseSender = mpsc::UnboundedSender<Result<serde_json::Value, WsError>>;
type ResponseReceiver = mpsc::UnboundedReceiver<Result<serde_json::Value, WsError>>;

/// Errors surfaced on a [`SubscriptionStream`].
#[derive(Debug, Clone, thiserror::Error)]
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
/// Speaks the `graphql-transport-ws` protocol directly on top of
/// [`tokio_tungstenite`] so that `ping` messages are sent on a fixed schedule
/// regardless of inbound traffic — required because `async-graphql` only
/// resets its `keepalive_timeout` when it receives a client protocol message.
#[derive(Debug, Clone)]
pub struct WsClient {
    url: Url,
}

/// A live WebSocket session that can host multiple concurrent subscriptions
/// over a single connection.
///
/// `Session` is cheap to clone (backed by an [`Arc`]); the underlying
/// connection is closed when the last clone is dropped and every outstanding
/// subscription stream has also been dropped.
#[derive(Debug, Clone)]
pub struct Session {
    inner: Arc<SessionInner>,
}

#[derive(Debug)]
struct SessionInner {
    commands: mpsc::UnboundedSender<Command>,
    next_id: AtomicU64,
}

impl Drop for SessionInner {
    fn drop(&mut self) {
        let _ = self.commands.unbounded_send(Command::Close);
    }
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

enum Command {
    Subscribe {
        id: String,
        payload: serde_json::Value,
        tx: ResponseSender,
    },
    Unsubscribe {
        id: String,
    },
    Close,
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

    /// Open a new WebSocket connection and return a [`Session`] that can host
    /// multiple concurrent subscriptions over the same connection.
    ///
    /// The handshake (`connection_init`/`connection_ack`) is performed before
    /// returning. The connection stays open as long as the `Session` (or a
    /// clone) or any subscription stream derived from it is alive.
    pub async fn connect(&self) -> Result<Session, anyhow::Error> {
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

        #[cfg(feature = "tracing")]
        tracing::debug!("WebSocket handshake complete");

        let (cmd_tx, cmd_rx) = mpsc::unbounded::<Command>();
        tokio::spawn(run_session(sink, stream, cmd_rx));

        Ok(Session {
            inner: Arc::new(SessionInner {
                commands: cmd_tx,
                next_id: AtomicU64::new(1),
            }),
        })
    }

    /// Open a new WebSocket connection and start a single subscription.
    ///
    /// Convenience wrapper around [`WsClient::connect`] followed by
    /// [`Session::subscribe`]; the underlying connection is dedicated to this
    /// subscription and closes automatically when the returned stream is
    /// dropped.
    ///
    /// Use [`WsClient::connect`] directly if you want to multiplex multiple
    /// subscriptions over a single WebSocket.
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
        Q::ResponseData: DeserializeOwned + Unpin + Send + Sync + 'static,
    {
        let session = self.connect().await?;
        session.subscribe::<Q>(variables).await
    }
}

impl Session {
    /// Start a new subscription on this session.
    ///
    /// Multiple subscriptions can run concurrently on the same session; each
    /// is tagged with a unique id at the protocol level and routed back to
    /// its own stream.
    pub async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<SubscriptionStream<Q::ResponseData>, anyhow::Error>
    where
        Q: GraphQLQuery + Unpin + Send + Sync + 'static,
        Q::Variables: Unpin + Send + Sync + 'static,
        Q::ResponseData: DeserializeOwned + Unpin + Send + Sync + 'static,
    {
        let body = Q::build_query(variables);
        let payload = serde_json::json!({
            "query": body.query,
            "variables": body.variables,
            "operationName": body.operation_name,
        });

        let id = self
            .inner
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .to_string();

        let (tx, rx) = mpsc::unbounded::<Result<serde_json::Value, WsError>>();

        self.inner
            .commands
            .unbounded_send(Command::Subscribe {
                id: id.clone(),
                payload,
                tx,
            })
            .map_err(|_| anyhow::anyhow!("session is closed"))?;

        #[cfg(feature = "tracing")]
        tracing::debug!(subscription_id = %id, "Subscription registered");

        Ok(Box::pin(SubscriptionStreamInner::<Q::ResponseData> {
            rx,
            id: Some(id),
            commands: self.inner.commands.clone(),
            _session: self.clone(),
            _phantom: PhantomData,
        }))
    }
}

struct SubscriptionStreamInner<T> {
    rx: ResponseReceiver,
    id: Option<String>,
    commands: mpsc::UnboundedSender<Command>,
    _session: Session,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Stream for SubscriptionStreamInner<T>
where
    T: DeserializeOwned + Unpin + Send + 'static,
{
    type Item = Result<Response<T>, WsError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.rx).poll_next(cx) {
            Poll::Ready(Some(Ok(payload))) => Poll::Ready(Some(
                serde_json::from_value::<Response<T>>(payload)
                    .map_err(|e| WsError::Decode(e.to_string())),
            )),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for SubscriptionStreamInner<T> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            let _ = self.commands.unbounded_send(Command::Unsubscribe { id });
        }
    }
}

async fn run_session(
    mut sink: SplitSink<WsStream, Message>,
    mut stream: SplitStream<WsStream>,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut senders: HashMap<String, ResponseSender> = HashMap::new();
    let mut ping_interval = tokio::time::interval(KEEP_ALIVE_INTERVAL);
    // First tick fires immediately; skip it so we don't ping right after the ack.
    ping_interval.tick().await;

    let ping_msg = serde_json::to_string(&ClientMessage::Ping)
        .expect("serializing unit enum variant never fails");
    let pong_msg = serde_json::to_string(&ClientMessage::Pong)
        .expect("serializing unit enum variant never fails");

    let exit_error = loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if sink.send(Message::text(ping_msg.clone())).await.is_err() {
                    break Some(WsError::Transport("failed to send ping".into()));
                }
            }
            cmd = commands.next() => match cmd {
                Some(Command::Subscribe { id, payload, tx }) => {
                    let msg = serde_json::to_string(&ClientMessage::Subscribe {
                        id: &id,
                        payload,
                    })
                    .expect("serializing subscribe never fails");
                    senders.insert(id, tx);
                    if sink.send(Message::text(msg)).await.is_err() {
                        break Some(WsError::Transport("failed to send subscribe".into()));
                    }
                }
                Some(Command::Unsubscribe { id }) => {
                    if senders.remove(&id).is_some() {
                        let msg = serde_json::to_string(&ClientMessage::Complete { id: &id })
                            .expect("serializing complete never fails");
                        if sink.send(Message::text(msg)).await.is_err() {
                            break Some(WsError::Transport("failed to send complete".into()));
                        }
                    }
                }
                Some(Command::Close) | None => break None,
            },
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        let decoded: ServerMessage = match serde_json::from_str(&txt) {
                            Ok(m) => m,
                            Err(e) => break Some(WsError::Decode(e.to_string())),
                        };

                        match decoded {
                            ServerMessage::Next { id, payload } => {
                                if let Some(tx) = senders.get(&id)
                                    && tx.unbounded_send(Ok(payload)).is_err()
                                {
                                    // Consumer dropped its stream without sending
                                    // Unsubscribe yet; drop the sender and tell the server.
                                    senders.remove(&id);
                                    let msg =
                                        serde_json::to_string(&ClientMessage::Complete { id: &id })
                                            .expect("serializing complete never fails");
                                    if sink.send(Message::text(msg)).await.is_err() {
                                        break Some(WsError::Transport(
                                            "failed to send complete".into(),
                                        ));
                                    }
                                }
                            }
                            ServerMessage::Error { id, payload } => {
                                if let Some(tx) = senders.remove(&id) {
                                    let _ = tx.unbounded_send(Err(WsError::Subscription(payload)));
                                }
                            }
                            ServerMessage::Complete { id } => {
                                // Dropping the sender closes the consumer's stream.
                                senders.remove(&id);
                            }
                            ServerMessage::Ping => {
                                if sink.send(Message::text(pong_msg.clone())).await.is_err() {
                                    break Some(WsError::Transport("failed to send pong".into()));
                                }
                            }
                            ServerMessage::Pong | ServerMessage::ConnectionAck => {},
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sink.send(Message::Pong(data)).await.is_err() {
                            break Some(WsError::Transport("failed to send pong".into()));
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| format!("{} (code {})", f.reason, u16::from(f.code)))
                            .unwrap_or_else(|| "no close frame".to_string());
                        break Some(WsError::Closed(reason));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => break Some(WsError::Transport(e.to_string())),
                    None => break Some(WsError::Closed("stream ended".into())),
                }
            }
        }
    };

    // Best-effort graceful shutdown.
    if let Some(err) = &exit_error {
        for tx in senders.values() {
            let _ = tx.unbounded_send(Err(err.clone()));
        }
    }
    for id in senders.keys() {
        let msg = serde_json::to_string(&ClientMessage::Complete { id })
            .expect("serializing complete never fails");
        let _ = sink.send(Message::text(msg)).await;
    }
    drop(senders);
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
        <<Self as Variables>::Query as GraphQLQuery>::ResponseData:
            DeserializeOwned + Unpin + Send + Sync + 'static,
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
