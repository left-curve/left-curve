//! End-to-end tests for the multiplexed WebSocket subscription endpoint
//! (`GET /ws`), which serves the validator's in-memory block / perps-event
//! feeds over a custom, `method`/`channel`-tagged JSON protocol.
//!
//! These run against a real TCP server (`actix_test::start`) and drive it with
//! a real WebSocket client (`TestServer::ws_at`), so the upgrade, the framing,
//! and the per-connection task are all exercised end to end.

use {
    actix_http::ws,
    anyhow::anyhow,
    dango_app::Indexer,
    dango_primitives::{Block, BlockInfo, FullBlock},
    dango_testing::{
        TestOption, build_app_service, create_perps_fill, pair_id, setup_perps_env,
        setup_test_naive_with_indexer,
    },
    futures_util::{SinkExt, Stream, StreamExt},
    serde_json::{Value, json},
    std::time::Duration,
};

/// Per-read idle budget. The in-memory snapshot arrives promptly and the live
/// tail is then silent, so a read that idles this long has drained the snapshot;
/// it is well below the 20s server keepalive ping, so no ping interferes.
const IDLE: Duration = Duration::from_secs(3);

/// Send one client message (a JSON object) as a text frame.
async fn send<S>(framed: &mut S, message: Value) -> anyhow::Result<()>
where
    S: SinkExt<ws::Message, Error = ws::ProtocolError> + Unpin,
{
    framed
        .send(ws::Message::Text(message.to_string().into()))
        .await
        .map_err(|err| anyhow!("ws send failed: {err:?}"))
}

/// Collect server messages until the stream idles past [`IDLE`], closes, or
/// `max` are gathered. Used to drain a subscription snapshot; never hangs,
/// because the live tail is silent.
async fn drain<S>(framed: &mut S, max: usize) -> anyhow::Result<Vec<Value>>
where
    S: Stream<Item = Result<ws::Frame, ws::ProtocolError>> + Unpin,
{
    let mut out = Vec::new();
    while out.len() < max {
        match tokio::time::timeout(IDLE, framed.next()).await {
            Err(_) | Ok(None) | Ok(Some(Ok(ws::Frame::Close(_)))) => break,
            Ok(Some(Ok(ws::Frame::Text(bytes)))) => out.push(serde_json::from_slice(&bytes)?),
            Ok(Some(Ok(_))) => {}, // ping/pong/binary/continuation
            Ok(Some(Err(err))) => return Err(anyhow!("ws frame error: {err:?}")),
        }
    }
    Ok(out)
}

/// Read until a server message satisfies `pred`, returning it, or error on idle.
async fn recv_until<S, F>(framed: &mut S, mut pred: F) -> anyhow::Result<Value>
where
    S: Stream<Item = Result<ws::Frame, ws::ProtocolError>> + Unpin,
    F: FnMut(&Value) -> bool,
{
    loop {
        match tokio::time::timeout(IDLE, framed.next()).await {
            Err(_) => return Err(anyhow!("timed out waiting for a matching frame")),
            Ok(None) | Ok(Some(Ok(ws::Frame::Close(_)))) => {
                return Err(anyhow!("socket closed before a matching frame"));
            },
            Ok(Some(Ok(ws::Frame::Text(bytes)))) => {
                let value: Value = serde_json::from_slice(&bytes)?;
                if pred(&value) {
                    return Ok(value);
                }
            },
            Ok(Some(Ok(_))) => {},
            Ok(Some(Err(err))) => return Err(anyhow!("ws frame error: {err:?}")),
        }
    }
}

fn channel(message: &Value) -> Option<&str> {
    message.get("channel").and_then(Value::as_str)
}

/// Subscribing to `blockInfo` and `block` on one socket replays the retained
/// window on each channel independently: both are acked, each frame parses into
/// exactly its primitive — a bare `BlockInfo` (no transactions), a `Block` (no
/// execution outcome); both types reject unknown fields, so parsing proves
/// nothing extra leaked in — and the two channels report the same ascending
/// heights, since they project the same ring.
#[tokio::test(flavor = "multi_thread")]
async fn ws_block_info_and_block_streams_replay_blocks() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let since = ctx.stream_context.blocks().floor().unwrap_or_default();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 8, "subscription": {"type": "blockInfo", "since": since}}),
                )
                .await?;
                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 9, "subscription": {"type": "block", "since": since}}),
                )
                .await?;

                let messages = drain(&mut framed, 256).await?;

                // Both subscriptions are acknowledged.
                for id in [8, 9] {
                    messages
                        .iter()
                        .find(|m| {
                            channel(m) == Some("subscriptionResponse")
                                && m["id"].as_u64() == Some(id)
                        })
                        .ok_or_else(|| anyhow!("no ack for subscription {id}; got {messages:?}"))?;
                }

                // `blockInfo` frames are tagged id = 8 and parse into a bare
                // `BlockInfo`, with ascending heights.
                let mut info_heights = Vec::new();
                for frame in messages.iter().filter(|m| channel(m) == Some("blockInfo")) {
                    assert_eq!(frame["id"].as_u64(), Some(8));
                    let info: BlockInfo = serde_json::from_value(frame["data"].clone())
                        .map_err(|err| anyhow!("frame data is not a BlockInfo: {err}"))?;
                    info_heights.push(info.height);
                }
                assert!(
                    info_heights.len() >= 2,
                    "expected at least two blockInfo frames; got {messages:?}"
                );
                assert!(
                    info_heights.windows(2).all(|w| w[1] > w[0]),
                    "blockInfo heights must ascend: {info_heights:?}"
                );

                // `block` frames are tagged id = 9 and parse into a `Block`.
                let mut block_heights = Vec::new();
                for frame in messages.iter().filter(|m| channel(m) == Some("block")) {
                    assert_eq!(frame["id"].as_u64(), Some(9));
                    let block: Block = serde_json::from_value(frame["data"].clone())
                        .map_err(|err| anyhow!("frame data is not a Block: {err}"))?;
                    block_heights.push(block.info.height);
                }

                // Both channels project the same ring, so they replay the same
                // height sequence.
                assert_eq!(info_heights, block_heights);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// Subscribing to `fullBlock` with a `since` replays the retained window then
/// holds the live tail: an ack arrives first, then ascending `fullBlock` frames,
/// each tagged with the subscription `id` and parsing back into a `FullBlock`.
#[tokio::test(flavor = "multi_thread")]
async fn ws_full_block_stream_replays_blocks() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let since = ctx.stream_context.blocks().floor().unwrap_or_default();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 7, "subscription": {"type": "fullBlock", "since": since}}),
                )
                .await?;

                let messages = drain(&mut framed, 256).await?;

                // First, the subscription is acknowledged.
                let ack = messages
                    .iter()
                    .find(|m| channel(m) == Some("subscriptionResponse"))
                    .ok_or_else(|| anyhow!("no subscriptionResponse ack; got {messages:?}"))?;
                assert_eq!(ack["id"].as_u64(), Some(7));

                // Then ascending `fullBlock` data frames, each tagged id = 7 and
                // parsing back into a `FullBlock`.
                let blocks: Vec<&Value> = messages
                    .iter()
                    .filter(|m| channel(m) == Some("fullBlock"))
                    .collect();
                assert!(blocks.len() >= 2, "expected at least two block frames; got {messages:?}");

                let mut heights = Vec::new();
                for frame in &blocks {
                    assert_eq!(frame["id"].as_u64(), Some(7));
                    let block: FullBlock = serde_json::from_value(frame["data"].clone())
                        .map_err(|err| anyhow!("frame data is not a FullBlock: {err}"))?;
                    heights.push(block.block.info.height);
                }
                assert!(
                    heights.windows(2).all(|w| w[1] > w[0]),
                    "block heights must ascend: {heights:?}"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// A `pairIds`-filtered `perpsEvents` subscription replays only blocks carrying
/// a matching event; every delivered event is for that pair, the frames are
/// tagged with the subscription `id`, and the payload carries the camelCase wire
/// shape (`blockHeight` / `eventType` / `pairId` / ...).
#[tokio::test(flavor = "multi_thread")]
async fn ws_perps_events_stream_filtered_by_pair() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let since = ctx.stream_context.perps().floor().unwrap_or_default();
    let pair_str = pair.to_string();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let pair_str = pair_str.clone();
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 1, "subscription": {"type": "perpsEvents", "since": since, "pairIds": [pair_str]}}),
                )
                .await?;

                let messages = drain(&mut framed, 256).await?;

                let batches: Vec<&Value> = messages
                    .iter()
                    .filter(|m| channel(m) == Some("perpsEvents"))
                    .collect();
                assert!(!batches.is_empty(), "expected at least one perps batch; got {messages:?}");

                let mut saw_order_filled = false;
                for frame in &batches {
                    assert_eq!(frame["id"].as_u64(), Some(1));
                    let data = &frame["data"];
                    // Wire shape: camelCase batch fields are present.
                    assert!(data.get("blockHeight").is_some(), "missing blockHeight: {data}");
                    assert!(data.get("createdAt").is_some(), "missing createdAt: {data}");
                    for event in data["events"].as_array().unwrap_or(&vec![]) {
                        assert_eq!(
                            event["pairId"].as_str(),
                            Some(pair_str.as_str()),
                            "pair filter leaked a non-matching event: {event}"
                        );
                        if event["eventType"].as_str() == Some("order_filled") {
                            saw_order_filled = true;
                        }
                    }
                }
                assert!(saw_order_filled, "the pair-filtered stream should replay order_filled");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// Omitting the filters (the WS analogue of GraphQL's absent arguments) matches
/// every event rather than suppressing them.
#[tokio::test(flavor = "multi_thread")]
async fn ws_perps_events_stream_absent_filter_matches_all() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let since = ctx.stream_context.perps().floor().unwrap_or_default();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 1, "subscription": {"type": "perpsEvents", "since": since}}),
                )
                .await?;

                let messages = drain(&mut framed, 256).await?;
                let total: usize = messages
                    .iter()
                    .filter(|m| channel(m) == Some("perpsEvents"))
                    .filter_map(|m| m["data"]["events"].as_array())
                    .map(Vec::len)
                    .sum();

                assert!(total > 0, "absent filters must match all events, not suppress them");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// A `since` older than the retained in-memory window does not open a stream; it
/// replies with an `error`-keyed frame tagged `resync` (carrying the offending
/// `id`) on the subscription's own `perpsEvents` channel — the WS analogue of
/// the SSE `409 Conflict`.
#[tokio::test(flavor = "multi_thread")]
async fn ws_perps_events_stream_resync_required_is_error_frame() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let floor = ctx
        .stream_context
        .perps()
        .floor()
        .expect("perps ring is non-empty after a fill");
    assert!(floor >= 1, "unexpected ring floor: {floor}");
    let stale = floor - 1;

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 3, "subscription": {"type": "perpsEvents", "since": stale}}),
                )
                .await?;

                let error = recv_until(&mut framed, |m| {
                    channel(m) == Some("perpsEvents") && m.get("error").is_some()
                })
                .await?;
                assert_eq!(error["id"].as_u64(), Some(3));
                assert_eq!(error["error"]["code"].as_str(), Some("resync"));

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// `ping` is answered with a `pong` carrying the same `id`, and `unsubscribe`
/// acknowledges the matching subscription — exercising the control protocol on a
/// single socket.
#[tokio::test(flavor = "multi_thread")]
async fn ws_ping_pong_and_unsubscribe() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut srv = actix_test::start(move || build_app_service(ctx.clone()));
                let mut framed = srv
                    .ws_at("/ws")
                    .await
                    .map_err(|err| anyhow!("ws upgrade failed: {err}"))?;

                // ping -> pong, id echoed.
                send(&mut framed, json!({"method": "ping", "id": 9})).await?;
                let pong = recv_until(&mut framed, |m| channel(m) == Some("pong")).await?;
                assert_eq!(pong["id"].as_u64(), Some(9));

                // subscribe -> ack, then unsubscribe -> ack.
                send(
                    &mut framed,
                    json!({"method": "subscribe", "id": 2, "subscription": {"type": "perpsEvents"}}),
                )
                .await?;
                let sub_ack = recv_until(&mut framed, |m| {
                    channel(m) == Some("subscriptionResponse") && m["id"].as_u64() == Some(2)
                })
                .await?;
                assert_eq!(sub_ack["data"]["method"].as_str(), Some("subscribe"));

                send(&mut framed, json!({"method": "unsubscribe", "id": 2})).await?;
                let unsub_ack = recv_until(&mut framed, |m| {
                    channel(m) == Some("subscriptionResponse")
                        && m["data"]["method"].as_str() == Some("unsubscribe")
                })
                .await?;
                assert_eq!(unsub_ack["id"].as_u64(), Some(2));

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
