//! Multiplexed WebSocket subscription endpoint (`GET /ws`).
//!
//! One socket carries any number of subscriptions, each identified by a
//! client-chosen `id`. Client messages are `method`-tagged
//! (`subscribe` / `unsubscribe` / `ping`); server messages are `channel`-tagged
//! (`subscriptionResponse` / `<channel>` data / `pong` / `error`). The `id` of a
//! `subscribe` is echoed on its acknowledgement and on every data frame it
//! produces, so concurrent subscriptions (e.g. several `perpsEvents` feeds with
//! different filters) are demultiplexable on the client.
//!
//! Two channel types are served, both reusing the in-memory validator stream
//! that backed the `full_block` / `perps_events2` GraphQL subscriptions:
//!
//! - `fullBlock` — every finalized block (`{block, outcome}`).
//! - `perpsEvents` — perps-contract events grouped per block, narrowed by the
//!   `eventTypes` / `pairIds` / `users` / `orderIds` / `clientOrderIds` filters.
//!
//! The transport is otherwise a thin shell over
//! [`dango_indexer_stream::Context`]; only the framing differs from the SSE and
//! GraphQL transports. The message enums are left open (no `#[non_exhaustive]`
//! barrier, but room in the protocol) so a future request/response write path
//! (`broadcast` / `post`) and further read channels (`l2Book`, `candle`, …) can
//! be added without breaking existing clients.

use {
    crate::{
        context::FullContext,
        subscription_limiter::{ConnectionLimiter, SubscriptionLimiter, guard_subscription_stream},
    },
    actix_web::{HttpRequest, HttpResponse, Resource, guard, web},
    actix_ws::{AggregatedMessage, Session},
    dango_indexer_stream::{Context, make_perps_filter},
    dango_primitives::FullBlock,
    futures_util::{StreamExt, stream},
    serde::{Deserialize, Serialize},
    std::{collections::HashSet, pin::Pin, sync::Arc, time::Duration},
    tokio::time::{Instant, MissedTickBehavior, interval},
    tokio_stream::StreamMap,
};

/// How often the server sends a WebSocket control-frame ping and re-checks the
/// idle deadline.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);

/// Close a socket that has not produced any inbound frame — including the
/// automatic pong to our keepalive ping — for this long. A live but idle
/// subscriber stays connected because its WebSocket stack pongs our pings; a
/// dead connection is reaped within `IDLE_TIMEOUT`.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Each active subscription's stream, already projected to ready-to-send JSON
/// text frames. Boxing erases the per-channel item type so both channels share
/// one [`StreamMap`]; `Pin<Box<_>>` is `Unpin`, as `StreamMap` requires.
type FrameStream = Pin<Box<dyn stream::Stream<Item = String> + Send>>;

/// Register the `GET /ws` upgrade route. The `upgrade: websocket` header guard
/// mirrors the GraphQL route, so a plain (non-upgrade) `GET /ws` falls through
/// to the default 404 handler instead of erroring inside the upgrade.
pub fn services() -> Resource {
    web::resource("/ws").route(
        web::get()
            .guard(guard::Header("upgrade", "websocket"))
            .to(ws_index),
    )
}

// ---- client and server message types ----

/// A message sent by the client. Discriminated by `method`.
#[derive(Debug, Deserialize)]
#[serde(
    tag = "method",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ClientMessage {
    /// Open a subscription. `id` is the client-chosen handle echoed on the
    /// acknowledgement and every resulting data frame, and used to
    /// `unsubscribe`.
    Subscribe {
        id: u64,
        subscription: Box<Subscription>,
    },
    /// Close the subscription opened with this `id`.
    Unsubscribe { id: u64 },
    /// Application-level heartbeat; answered with a `pong` carrying the same
    /// `id`.
    Ping {
        #[serde(default)]
        id: Option<u64>,
    },
}

/// The feed a `subscribe` selects, discriminated by `type`. The boxed
/// `perpsEvents` parameters keep the variants similarly sized.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Subscription {
    PerpsEvents(Box<PerpsEventsParams>),
    FullBlock {
        #[serde(default)]
        since: Option<u64>,
    },
}

/// Filters for a `perpsEvents` subscription, mirroring the former
/// `perps_events2` GraphQL arguments: an absent/`null` set does not filter on
/// that field; an empty set matches nothing; the sets AND together.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpsEventsParams {
    #[serde(default)]
    since: Option<u64>,
    #[serde(default)]
    event_types: Option<HashSet<String>>,
    #[serde(default)]
    pair_ids: Option<HashSet<String>>,
    #[serde(default)]
    users: Option<HashSet<String>>,
    #[serde(default)]
    order_ids: Option<HashSet<String>>,
    #[serde(default)]
    client_order_ids: Option<HashSet<String>>,
}

impl Subscription {
    /// The channel name data frames from this subscription carry.
    fn channel(&self) -> &'static str {
        match self {
            Subscription::PerpsEvents(_) => "perpsEvents",
            Subscription::FullBlock { .. } => "fullBlock",
        }
    }
}

/// A control message sent by the server. Discriminated by `channel`. Data frames
/// are serialized separately via [`DataFrame`] because their payload type varies
/// per channel.
#[derive(Debug, Serialize)]
#[serde(tag = "channel", rename_all = "camelCase")]
enum ServerControl<'a> {
    SubscriptionResponse {
        id: u64,
        data: Ack<'a>,
    },
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        data: ErrorData<'a>,
    },
}

#[derive(Debug, Serialize)]
struct Ack<'a> {
    /// `"subscribe"` or `"unsubscribe"`.
    method: &'a str,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    channel: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ErrorData<'a> {
    code: &'a str,
    message: &'a str,
}

/// One block's worth of data on a channel, tagged with its source subscription
/// `id`.
#[derive(Debug, Serialize)]
struct DataFrame<'a, T> {
    channel: &'static str,
    id: u64,
    data: &'a T,
}

// ---- handler ----

/// Accept the WebSocket upgrade and drive the connection on a detached task.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn ws_index(
    req: HttpRequest,
    body: web::Payload,
    app_ctx: web::Data<FullContext>,
    limiter: web::Data<SubscriptionLimiter>,
) -> actix_web::Result<HttpResponse> {
    let (response, session, msg_stream) = actix_ws::handle(&req, body)?;

    let stream_ctx = app_ctx.stream_context.clone();
    let conn_limiter = limiter.new_connection();

    actix_web::rt::spawn(connection_loop(
        session,
        msg_stream,
        stream_ctx,
        conn_limiter,
    ));

    Ok(response)
}

/// The per-connection event loop: multiplex inbound control messages, active
/// subscription streams, and the keepalive timer over one socket.
async fn connection_loop(
    mut session: Session,
    msg_stream: actix_ws::MessageStream,
    stream_ctx: Context,
    conn_limiter: ConnectionLimiter,
) {
    let mut msg_stream = msg_stream.aggregate_continuations();
    let mut streams: StreamMap<u64, FrameStream> = StreamMap::new();

    let mut keepalive = interval(KEEPALIVE_INTERVAL);
    keepalive.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_seen = Instant::now();

    loop {
        tokio::select! {
            // Inbound control messages from the client.
            inbound = msg_stream.next() => {
                let Some(Ok(message)) = inbound else { break };
                last_seen = Instant::now();
                match message {
                    AggregatedMessage::Text(text) => {
                        if !handle_text(&text, &mut session, &mut streams, &stream_ctx, &conn_limiter).await {
                            break;
                        }
                    },
                    AggregatedMessage::Ping(bytes) => {
                        if session.pong(&bytes).await.is_err() {
                            break;
                        }
                    },
                    AggregatedMessage::Close(_) => break,
                    // Pong (answer to our keepalive) and Binary are no-ops.
                    AggregatedMessage::Pong(_) | AggregatedMessage::Binary(_) => {},
                }
            },

            // A subscription produced a frame. The `if` guard disables this arm
            // while the map is empty, since `StreamMap::next` resolves to `None`
            // immediately on an empty map.
            Some((_id, frame)) = streams.next(), if !streams.is_empty() => {
                if session.text(frame).await.is_err() {
                    break;
                }
            },

            // Keepalive + idle reaping.
            _ = keepalive.tick() => {
                if last_seen.elapsed() > IDLE_TIMEOUT {
                    let _ = session.close(None).await;
                    break;
                }
                if session.ping(b"").await.is_err() {
                    break;
                }
            },
        }
    }
}

/// Handle one inbound text frame. Returns `false` when the socket is gone and
/// the loop should stop; protocol-level problems are reported as `error` frames
/// and return `true`.
async fn handle_text(
    text: &str,
    session: &mut Session,
    streams: &mut StreamMap<u64, FrameStream>,
    stream_ctx: &Context,
    conn_limiter: &ConnectionLimiter,
) -> bool {
    let message = match serde_json::from_str::<ClientMessage>(text) {
        Ok(message) => message,
        Err(err) => return send(session, error_frame(None, "badRequest", &err.to_string())).await,
    };

    match message {
        ClientMessage::Subscribe { id, subscription } => {
            if streams.contains_key(&id) {
                return send(
                    session,
                    error_frame(Some(id), "badRequest", "id already in use"),
                )
                .await;
            }

            // Reserve a subscription slot; the guard rides inside the stream and
            // releases the slot when the stream is dropped (unsubscribe or
            // disconnect).
            let guard = match conn_limiter.try_acquire() {
                Ok(guard) => Arc::new(guard),
                Err(err) => {
                    return send(
                        session,
                        error_frame(Some(id), "tooManyRequests", &err.message),
                    )
                    .await;
                },
            };

            let channel = subscription.channel();
            match open_stream(id, &subscription, guard, stream_ctx) {
                Ok(frames) => {
                    streams.insert(id, frames);
                    send(session, ack(id, "subscribe", Some(channel))).await
                },
                Err(resync) => send(session, error_frame(Some(id), "resync", &resync)).await,
            }
        },
        ClientMessage::Unsubscribe { id } => {
            if streams.remove(&id).is_some() {
                send(session, ack(id, "unsubscribe", None)).await
            } else {
                send(
                    session,
                    error_frame(Some(id), "unknownSubscription", "no such id"),
                )
                .await
            }
        },
        ClientMessage::Ping { id } => send(session, control(&ServerControl::Pong { id })).await,
    }
}

/// Open the validator stream for a subscription and project it to tagged JSON
/// text frames. The `guard` is moved into the projected stream so it lives
/// exactly as long as the subscription. On a connect-time resync (the requested
/// `since` predates the retained window) the guard is dropped and the resync
/// message returned as `Err`.
fn open_stream(
    id: u64,
    subscription: &Subscription,
    guard: Arc<crate::subscription_limiter::SubscriptionGuard>,
    stream_ctx: &Context,
) -> Result<FrameStream, String> {
    match subscription {
        Subscription::PerpsEvents(params) => {
            let filter = make_perps_filter(
                params.event_types.clone(),
                params.pair_ids.clone(),
                params.users.clone(),
                params.order_ids.clone(),
                params.client_order_ids.clone(),
            );
            let raw = stream_ctx
                .perps()
                .subscribe(params.since, filter)
                .map_err(|resync| resync.to_string())?;
            let frames = guard_subscription_stream(raw, Some(guard))
                .map(move |block| data_frame("perpsEvents", id, &block));
            Ok(with_terminal(id, frames))
        },
        Subscription::FullBlock { since } => {
            let raw = stream_ctx
                .blocks()
                .subscribe(*since, |block: &FullBlock| Some(block.clone()))
                .map_err(|resync| resync.to_string())?;
            let frames = guard_subscription_stream(raw, Some(guard))
                .map(move |block| data_frame("fullBlock", id, &block));
            Ok(with_terminal(id, frames))
        },
    }
}

/// Append a terminal `error` frame so that when a subscription ends on its own
/// — the validator window evicted the next block the subscriber needed, or the
/// feed closed — the client learns to resync instead of seeing the feed go
/// silent. An explicit `unsubscribe` removes the stream from the map before this
/// runs, so it never fires spuriously.
fn with_terminal<S>(id: u64, frames: S) -> FrameStream
where
    S: stream::Stream<Item = String> + Send + 'static,
{
    let terminal = stream::once(async move {
        error_frame(
            Some(id),
            "resync",
            "subscription ended; reconnect with a newer `since`",
        )
    });
    Box::pin(frames.chain(terminal))
}

// ---- serialization helpers ----

async fn send(session: &mut Session, text: String) -> bool {
    session.text(text).await.is_ok()
}

fn control(message: &ServerControl) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| {
        r#"{"channel":"error","data":{"code":"internal","message":"serialization failed"}}"#
            .to_string()
    })
}

fn ack(id: u64, method: &str, channel: Option<&str>) -> String {
    control(&ServerControl::SubscriptionResponse {
        id,
        data: Ack { method, channel },
    })
}

fn error_frame(id: Option<u64>, code: &str, message: &str) -> String {
    control(&ServerControl::Error {
        id,
        data: ErrorData { code, message },
    })
}

fn data_frame<T>(channel: &'static str, id: u64, data: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(&DataFrame { channel, id, data })
        .unwrap_or_else(|err| error_frame(Some(id), "internal", &err.to_string()))
}

#[cfg(test)]
mod tests {
    use {super::*, serde_json::json};

    #[test]
    fn deserializes_subscribe_perps_events_with_filters() {
        let message: ClientMessage = serde_json::from_str(
            r#"{"method":"subscribe","id":1,"subscription":{"type":"perpsEvents","since":42,"eventTypes":["order_filled"],"pairIds":["perp/btcusd"]}}"#,
        )
        .unwrap();

        let ClientMessage::Subscribe { id, subscription } = message else {
            panic!("expected a subscribe message");
        };
        assert_eq!(id, 1);
        assert_eq!(subscription.channel(), "perpsEvents");

        let Subscription::PerpsEvents(params) = *subscription else {
            panic!("expected a perpsEvents subscription");
        };
        assert_eq!(params.since, Some(42));
        assert_eq!(
            params.event_types,
            Some(HashSet::from(["order_filled".to_string()]))
        );
        assert_eq!(
            params.pair_ids,
            Some(HashSet::from(["perp/btcusd".to_string()]))
        );
        // Absent filters deserialize to `None` (match-all).
        assert_eq!(params.users, None);
        assert_eq!(params.order_ids, None);
        assert_eq!(params.client_order_ids, None);
    }

    #[test]
    fn empty_filter_is_match_nothing_absent_is_match_all() {
        let message: ClientMessage = serde_json::from_str(
            r#"{"method":"subscribe","id":1,"subscription":{"type":"perpsEvents","pairIds":[]}}"#,
        )
        .unwrap();
        let ClientMessage::Subscribe { subscription, .. } = message else {
            panic!("expected a subscribe message");
        };
        let Subscription::PerpsEvents(params) = *subscription else {
            panic!("expected a perpsEvents subscription");
        };
        // An empty set matches nothing; an absent one is `None` (match-all).
        assert_eq!(params.pair_ids, Some(HashSet::new()));
        assert_eq!(params.users, None);
    }

    #[test]
    fn deserializes_full_block_unsubscribe_and_ping() {
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(
                r#"{"method":"subscribe","id":2,"subscription":{"type":"fullBlock"}}"#,
            )
            .unwrap(),
            ClientMessage::Subscribe { id: 2, .. }
        ));
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(r#"{"method":"unsubscribe","id":2}"#).unwrap(),
            ClientMessage::Unsubscribe { id: 2 }
        ));
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(r#"{"method":"ping","id":9}"#).unwrap(),
            ClientMessage::Ping { id: Some(9) }
        ));
        // `id` is optional on ping.
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(r#"{"method":"ping"}"#).unwrap(),
            ClientMessage::Ping { id: None }
        ));
    }

    #[test]
    fn serializes_control_frames() {
        let parse = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap();

        assert_eq!(
            parse(&ack(1, "subscribe", Some("perpsEvents"))),
            json!({"channel": "subscriptionResponse", "id": 1, "data": {"method": "subscribe", "type": "perpsEvents"}}),
        );
        // The unsubscribe ack omits the channel `type`.
        assert_eq!(
            parse(&ack(2, "unsubscribe", None)),
            json!({"channel": "subscriptionResponse", "id": 2, "data": {"method": "unsubscribe"}}),
        );
        assert_eq!(
            parse(&control(&ServerControl::Pong { id: Some(9) })),
            json!({"channel": "pong", "id": 9}),
        );
        // A null `id` is omitted, not serialized.
        assert_eq!(
            parse(&control(&ServerControl::Pong { id: None })),
            json!({"channel": "pong"}),
        );
        assert_eq!(
            parse(&error_frame(Some(3), "resync", "stale")),
            json!({"channel": "error", "id": 3, "data": {"code": "resync", "message": "stale"}}),
        );
    }

    #[test]
    fn serializes_data_frame_with_subscription_id() {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&data_frame(
                "perpsEvents",
                1,
                &json!({"blockHeight": 5}),
            ))
            .unwrap(),
            json!({"channel": "perpsEvents", "id": 1, "data": {"blockHeight": 5}}),
        );
    }
}
