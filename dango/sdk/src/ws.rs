use {
    anyhow::{anyhow, bail},
    dango_primitives::{BroadcastTxOutcome, Tx},
    futures::{
        SinkExt, Stream, StreamExt,
        channel::{mpsc, oneshot},
        stream::{SplitSink, SplitStream},
    },
    reqwest::IntoUrl,
    serde::{Deserialize, de::DeserializeOwned},
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
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message},
    url::Url,
};

/// App-level keepalive interval. The server closes a connection it has not heard
/// from for 60s; a 20s ping keeps an idle connection alive with a comfortable
/// margin (any inbound frame resets the server's idle timer).
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(20);

/// The WebSocket connection [`connect_async`] yields for a `ws(s)://` URL.
type WsConn = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Data frames delivered to a subscription: `Ok` carries the frame's `data`
/// payload, `Err` is a terminal error (the stream ends after it).
type FrameSender = mpsc::UnboundedSender<anyhow::Result<serde_json::Value>>;
type FrameReceiver = mpsc::UnboundedReceiver<anyhow::Result<serde_json::Value>>;

// ---- perps events data types ----

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

// ---- the handle ----

/// A single, long-lived native-`/ws` connection that multiplexes any number of
/// subscriptions (`perpsEvents`) **and** transaction broadcasts over one socket,
/// demultiplexed by the protocol's `id`/`channel`.
///
/// This is the shared-connection path a latency-sensitive bot wants: subscribe,
/// react, and broadcast all ride the same socket, so a `broadcast` never opens a
/// second connection and the event feed keeps draining while a broadcast is in
/// flight. The one-shot `HttpClient` helpers (REST `broadcast_tx`, the GraphQL
/// subscriptions) remain the simple per-operation path.
///
/// `WsConnection` is a cheap-to-`Clone` handle (backed by an [`Arc`]); it owns
/// only the sending end of a command channel. The socket and the routing
/// registries live in a background actor task ([`WsManager`]), reached only by
/// message-passing, so there are no locks on the hot path. The socket closes
/// once the last `WsConnection` clone and every [`Subscription`] have dropped.
///
/// Not to be confused with the graphql-transport-ws [`crate::WsClient`] /
/// [`crate::Session`], which speak a different protocol against `/graphql`.
#[derive(Debug, Clone)]
pub struct WsConnection {
    inner: Arc<WsConnectionInner>,
}

#[derive(Debug)]
struct WsConnectionInner {
    commands: mpsc::UnboundedSender<Command>,
    next_id: AtomicU64,
}

impl Drop for WsConnectionInner {
    fn drop(&mut self) {
        let _ = self.commands.unbounded_send(Command::Close);
    }
}

enum Command {
    Subscribe {
        id: u64,
        message: serde_json::Value,
        tx: FrameSender,
    },
    Unsubscribe {
        id: u64,
    },
    Broadcast {
        id: u64,
        message: serde_json::Value,
        reply: oneshot::Sender<anyhow::Result<BroadcastTxOutcome>>,
    },
    Close,
}

impl WsConnection {
    /// Open a `/ws` connection and spawn its background actor.
    ///
    /// `url` is the HTTP base (e.g. `https://api.dango.zone`); the `ws(s)://…/ws`
    /// endpoint is derived from it. Native `/ws` has no handshake, so this just
    /// splits the socket and spawns the manager.
    pub async fn connect<U>(url: U) -> anyhow::Result<Self>
    where
        U: IntoUrl,
    {
        let ws_url = to_ws_url(url.into_url()?)?;

        let (ws, _response) = connect_async(ws_url.as_str())
            .await
            .map_err(|err| anyhow!("WebSocket connection failed: {err}"))?;

        let (sink, stream) = ws.split();
        let (commands, command_rx) = mpsc::unbounded::<Command>();

        tokio::spawn(
            WsManager {
                sink,
                stream,
                commands: command_rx,
                subs: HashMap::new(),
                pending: HashMap::new(),
            }
            .run(),
        );

        Ok(Self {
            inner: Arc::new(WsConnectionInner {
                commands,
                next_id: AtomicU64::new(1),
            }),
        })
    }

    /// Subscribe to perps-exchange events over the shared socket (`perpsEvents`
    /// channel). Yields one [`PerpsEventsBatch`] per block that has at least one
    /// matching event; a terminal `Err` (e.g. the server's `resync` /
    /// `tooManyRequests`) ends the stream.
    ///
    /// The five filters are sets that AND together; `None` (or an empty list)
    /// does not filter on that field. `since_block_height` replays the retained
    /// in-memory window from that height (inclusive) before the live tail.
    pub fn subscribe_perps_events(
        &self,
        since_block_height: Option<u64>,
        event_types: Option<Vec<String>>,
        pair_ids: Option<Vec<String>>,
        users: Option<Vec<String>>,
        order_ids: Option<Vec<String>>,
        client_order_ids: Option<Vec<String>>,
    ) -> Subscription<PerpsEventsBatch> {
        // Absent filters are omitted (match-all); present ones are sent as JSON
        // arrays. Mirrors the server's `perpsEvents` subscription selector.
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

        self.subscribe(serde_json::Value::Object(subscription))
    }

    /// Broadcast a signed transaction over the shared socket (`broadcast`
    /// channel) and await its receipt — no second connection, and the event feed
    /// keeps draining while this awaits. Reply frames are correlated to requests
    /// by `id`, so several in-flight broadcasts never collide.
    ///
    /// As with REST, a mempool-rejected tx returns `Ok` (the rejection rides
    /// [`BroadcastTxOutcome::check_tx`]); only a transport failure is `Err`.
    pub async fn broadcast(&self, tx: Tx) -> anyhow::Result<BroadcastTxOutcome> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply, rx) = oneshot::channel();

        let message = serde_json::json!({
            "method": "broadcast",
            "id": id,
            "tx": serde_json::to_value(&tx)?,
        });

        self.inner
            .commands
            .unbounded_send(Command::Broadcast { id, message, reply })
            .map_err(|_| anyhow!("connection closed"))?;

        rx.await.map_err(|_| anyhow!("connection closed"))?
    }

    /// Allocate an id, register a frame channel, and send the subscribe command.
    /// Runs on the caller: no socket or registry access happens here.
    fn subscribe<T>(&self, subscription: serde_json::Value) -> Subscription<T>
    where
        T: DeserializeOwned,
    {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded::<anyhow::Result<serde_json::Value>>();

        let message = serde_json::json!({
            "method": "subscribe",
            "id": id,
            "subscription": subscription,
        });

        let _ = self
            .inner
            .commands
            .unbounded_send(Command::Subscribe { id, message, tx });

        Subscription {
            rx,
            id: Some(id),
            commands: self.inner.commands.clone(),
            _conn: self.clone(),
            _phantom: PhantomData,
        }
    }
}

// ---- the subscription stream ----

/// A [`Stream`] of one subscription's frames (the Rust analog of the Python
/// SDK's `Subscription` iterator). Yields `Ok(T)` per data frame and a terminal
/// `Err` on a subscription error or a closed connection; dropping it sends an
/// `unsubscribe` for its id.
pub struct Subscription<T> {
    rx: FrameReceiver,
    id: Option<u64>,
    commands: mpsc::UnboundedSender<Command>,
    // Keeps the connection alive while the stream lives, mirroring the socket's
    // RAII close: the actor exits only once every handle and stream have dropped.
    _conn: WsConnection,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Stream for Subscription<T>
where
    T: DeserializeOwned + Unpin,
{
    type Item = anyhow::Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.rx).poll_next(cx) {
            Poll::Ready(Some(Ok(value))) => {
                Poll::Ready(Some(serde_json::from_value::<T>(value).map_err(Into::into)))
            },
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            let _ = self.commands.unbounded_send(Command::Unsubscribe { id });
        }
    }
}

// ---- the actor ----

/// The single owner of the socket and the routing registries. Reached only
/// through the command channel; never held by the caller. Runs on a spawned
/// task, so its writes and reads run concurrently with a comfortable keepalive.
struct WsManager {
    sink: SplitSink<WsConn, Message>,
    stream: SplitStream<WsConn>,
    commands: mpsc::UnboundedReceiver<Command>,
    /// Live subscriptions, by id → frame sender.
    subs: HashMap<u64, FrameSender>,
    /// In-flight broadcasts, by id → one-shot reply.
    pending: HashMap<u64, oneshot::Sender<anyhow::Result<BroadcastTxOutcome>>>,
}

impl WsManager {
    async fn run(self) {
        // Destructure into locals so the `select!` can borrow the socket ends,
        // the command channel, and the registries independently.
        let Self {
            mut sink,
            mut stream,
            mut commands,
            mut subs,
            mut pending,
        } = self;

        let mut keepalive = tokio::time::interval(KEEP_ALIVE_INTERVAL);
        // First tick is immediate; skip it so we don't ping right after connect.
        keepalive.tick().await;
        let ping = serde_json::json!({ "method": "ping" }).to_string();

        let reason = loop {
            tokio::select! {
                _ = keepalive.tick() => {
                    if sink.send(Message::text(ping.clone())).await.is_err() {
                        break "keepalive send failed";
                    }
                }
                cmd = commands.next() => match cmd {
                    Some(Command::Subscribe { id, message, tx }) => {
                        subs.insert(id, tx);
                        if sink.send(Message::text(message.to_string())).await.is_err() {
                            break "subscribe send failed";
                        }
                    }
                    Some(Command::Unsubscribe { id }) => {
                        subs.remove(&id);
                        let msg = serde_json::json!({ "method": "unsubscribe", "id": id });
                        if sink.send(Message::text(msg.to_string())).await.is_err() {
                            break "unsubscribe send failed";
                        }
                    }
                    Some(Command::Broadcast { id, message, reply }) => {
                        pending.insert(id, reply);
                        if sink.send(Message::text(message.to_string())).await.is_err() {
                            break "broadcast send failed";
                        }
                    }
                    // The last handle dropped (or an explicit close): shut down.
                    Some(Command::Close) | None => break "connection closed",
                },
                frame = stream.next() => match frame {
                    Some(Ok(Message::Text(text))) => dispatch(&mut subs, &mut pending, &text),
                    Some(Ok(Message::Ping(data))) => {
                        if sink.send(Message::Pong(data)).await.is_err() {
                            break "pong send failed";
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break "socket closed",
                    Some(Ok(_)) => {},
                },
            }
        };

        // Notify every outstanding subscription and broadcast so nobody hangs.
        for (_, tx) in subs.drain() {
            let _ = tx.unbounded_send(Err(anyhow!("{reason}")));
        }
        for (_, reply) in pending.drain() {
            let _ = reply.send(Err(anyhow!("{reason}")));
        }
        let _ = sink.close().await;
    }
}

/// Route one inbound text frame to the subscription or broadcast that owns its
/// `id`. An `error` key is co-located on the operation's own channel and is
/// terminal for that operation.
fn dispatch(
    subs: &mut HashMap<u64, FrameSender>,
    pending: &mut HashMap<u64, oneshot::Sender<anyhow::Result<BroadcastTxOutcome>>>,
    text: &str,
) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };
    let id = value.get("id").and_then(serde_json::Value::as_u64);

    match value.get("channel").and_then(serde_json::Value::as_str) {
        // Mempool rejection is a `data` frame; only a transport failure to the
        // node is an `error` frame. Either way the broadcast is one-shot.
        Some("broadcast") => {
            if let Some(id) = id
                && let Some(reply) = pending.remove(&id)
            {
                let result = match value.get("error") {
                    Some(error) => Err(anyhow!("{}", error_detail(error))),
                    None => {
                        let data = value.get("data").cloned().unwrap_or_default();
                        serde_json::from_value::<BroadcastTxOutcome>(data).map_err(Into::into)
                    },
                };
                let _ = reply.send(result);
            }
        },
        Some("perpsEvents" | "fullBlock") => {
            if let Some(id) = id {
                match value.get("error") {
                    // Terminal: drop the registration and end the stream.
                    Some(error) => {
                        if let Some(tx) = subs.remove(&id) {
                            let _ = tx.unbounded_send(Err(anyhow!("{}", error_detail(error))));
                        }
                    },
                    None => {
                        if let Some(tx) = subs.get(&id) {
                            let data = value.get("data").cloned().unwrap_or_default();
                            let _ = tx.unbounded_send(Ok(data));
                        }
                    },
                }
            }
        },
        // `subscriptionResponse` / `pong` / a connection-level `error` (no id)
        // route to no single caller, so they are ignored.
        _ => {},
    }
}

/// Render a server `error` value (`{code, message}`) as `code: message`.
fn error_detail(error: &serde_json::Value) -> String {
    let code = error
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("error");
    let message = error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    format!("{code}: {message}")
}

/// Derive the `ws(s)://…/ws` endpoint from an HTTP base URL.
fn to_ws_url(base: Url) -> anyhow::Result<Url> {
    let mut ws_url = base.join("ws")?;
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

    Ok(ws_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_derivation() {
        // http(s) map to ws(s), and the `/ws` path is appended to the base.
        for (base, expected) in [
            ("https://api.dango.zone", "wss://api.dango.zone/ws"),
            ("https://api.dango.zone/", "wss://api.dango.zone/ws"),
            ("http://localhost:8080", "ws://localhost:8080/ws"),
            ("ws://localhost:8080", "ws://localhost:8080/ws"),
        ] {
            let url = to_ws_url(Url::parse(base).unwrap()).unwrap();
            assert_eq!(url.as_str(), expected, "base={base}");
        }
    }

    #[test]
    fn ws_url_rejects_unknown_scheme() {
        assert!(to_ws_url(Url::parse("ftp://example.com").unwrap()).is_err());
    }
}
