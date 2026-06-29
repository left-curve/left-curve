//! End-to-end tests for the REST/SSE subscription endpoints
//! (`GET /block/full/stream` and `GET /perps/events/stream`), which mirror the
//! `full_block` and `perps_events2` GraphQL subscriptions over plain
//! `text/event-stream`.
//!
//! These run against a real TCP server (`actix_test::start`) so the global
//! `Compress` middleware is in the loop: if the `Content-Encoding: identity`
//! bypass were missing, `Compress` would buffer the (endless) stream and the
//! reads below would time out instead of returning frames promptly.

use {
    actix_web::{
        http::{StatusCode, header},
        web::Bytes,
    },
    anyhow::anyhow,
    dango_app::Indexer,
    dango_indexer_graphql_types::subscribe_perps_events2,
    dango_primitives::FullBlock,
    dango_testing::{
        TestOption, build_app_service, create_perps_fill, pair_id, setup_perps_env,
        setup_test_naive_with_indexer,
    },
    futures_util::{Stream, StreamExt},
    std::time::Duration,
};

/// Per-frame read budget. Comfortably above the time the in-memory snapshot
/// takes to arrive, and below the 15s SSE keep-alive, so the read returns the
/// snapshot then breaks on the (idle) live tail rather than hanging.
const IDLE: Duration = Duration::from_secs(5);

/// One parsed SSE event: its `id` (block height) and its `data` payload (JSON).
type SseEvent = (Option<u64>, String);

/// Read SSE frames until `want` data events are collected, or the stream idles
/// past [`IDLE`] / closes — whichever comes first. Never hangs: the live tail is
/// endless, so callers rely on the idle break to terminate the snapshot read.
async fn read_sse_events<S, E>(mut body: S, want: usize) -> anyhow::Result<Vec<SseEvent>>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::fmt::Debug,
{
    let mut buf: Vec<u8> = Vec::new();
    let mut out: Vec<SseEvent> = Vec::new();

    while out.len() < want {
        match tokio::time::timeout(IDLE, body.next()).await {
            // Idle (no more snapshot, live tail silent) or stream closed.
            Err(_) | Ok(None) => break,
            Ok(Some(Err(err))) => return Err(anyhow!("SSE body error: {err:?}")),
            Ok(Some(Ok(chunk))) => {
                buf.extend_from_slice(&chunk);

                // Frames are delimited by a blank line. The delimiter is ASCII,
                // so each drained frame is a complete, valid UTF-8 unit.
                while let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
                    let frame = String::from_utf8(buf.drain(..pos + 2).collect())?;

                    let mut id = None;
                    let mut data = None;
                    for line in frame.lines() {
                        if let Some(rest) = line.strip_prefix("data:") {
                            data = Some(rest.strip_prefix(' ').unwrap_or(rest).to_string());
                        } else if let Some(rest) = line.strip_prefix("id:") {
                            id = rest.strip_prefix(' ').unwrap_or(rest).parse::<u64>().ok();
                        }
                        // `:` keep-alive comments and other fields are ignored.
                    }

                    if let Some(data) = data {
                        out.push((id, data));
                    }
                }
            },
        }
    }

    Ok(out)
}

/// `GET /block/full/stream` replays the retained window from `since` then holds
/// the live tail open: heights ascend, each frame is a `FullBlock`, and the
/// response is an uncompressed `text/event-stream` (proving the `Compress`
/// bypass — otherwise this read would hang).
#[tokio::test(flavor = "multi_thread")]
async fn block_full_stream_replays_blocks() -> anyhow::Result<()> {
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
                let srv = actix_test::start(move || build_app_service(ctx.clone()));

                let res = srv
                    .get(format!("/block/full/stream?since={since}"))
                    .send()
                    .await
                    .map_err(|err| anyhow!("request failed: {err}"))?;

                assert_eq!(res.status(), StatusCode::OK);
                assert_eq!(
                    res.headers()
                        .get(header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok()),
                    Some("text/event-stream"),
                );
                // If the server declared an encoding, it must be `identity` (the
                // Compress bypass), never a compression codec.
                if let Some(encoding) = res.headers().get(header::CONTENT_ENCODING) {
                    assert_eq!(encoding.to_str().ok(), Some("identity"));
                }

                let events = read_sse_events(res, 2).await?;

                let heights: Vec<u64> = events.iter().filter_map(|(id, _)| *id).collect();
                assert_eq!(heights.len(), 2, "expected two block frames");
                assert!(
                    heights[1] > heights[0],
                    "block heights must ascend: {heights:?}"
                );

                // Each frame is exactly a `FullBlock` (same shape as the REST
                // `/block/full/{height}` route and the `full_block` subscription).
                for (_, data) in &events {
                    let _: FullBlock = serde_json::from_str(data)
                        .map_err(|err| anyhow!("frame is not a FullBlock: {err}; body={data}"))?;
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// `GET /perps/events/stream?pairIds=<pair>` replays only the blocks carrying a
/// matching event, every event is for that pair, and the wire shape parses back
/// into the GraphQL `perps_events2` batch type — locking byte-shape parity for
/// the SDK substitution.
#[tokio::test(flavor = "multi_thread")]
async fn perps_events_stream_filtered_by_pair() -> anyhow::Result<()> {
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
            tokio::task::spawn_local(async move {
                let srv = actix_test::start({
                    let ctx = ctx.clone();
                    move || build_app_service(ctx.clone())
                });

                let res = srv
                    .get(format!(
                        "/perps/events/stream?since={since}&pairIds={pair_str}"
                    ))
                    .send()
                    .await
                    .map_err(|err| anyhow!("request failed: {err}"))?;

                assert_eq!(res.status(), StatusCode::OK);

                // Drain the snapshot (high cap; the idle break ends it).
                let events = read_sse_events(res, 256).await?;
                assert!(!events.is_empty(), "expected at least one perps batch");

                let mut saw_order_filled = false;
                for (_, data) in &events {
                    // Parses into the GraphQL `perps_events2` batch type => the
                    // SSE JSON matches the GraphQL wire shape field-for-field.
                    let batch: subscribe_perps_events2::SubscribePerpsEvents2PerpsEvents2 =
                        serde_json::from_str(data).map_err(|err| {
                            anyhow!("perps frame not parity with GraphQL type: {err}; body={data}")
                        })?;

                    for event in &batch.events {
                        assert_eq!(
                            event.pair_id.as_deref(),
                            Some(pair_str.as_str()),
                            "pair filter leaked a non-matching event"
                        );
                        if event.event_type == "order_filled" {
                            saw_order_filled = true;
                        }
                    }
                }

                assert!(
                    saw_order_filled,
                    "the pair-filtered stream should replay the order_filled event"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// An empty filter parameter (`?eventTypes=`) is treated the same as an absent
/// one — it does not suppress anything (the "comma-separated, simplified"
/// semantics: absent or empty both mean match-all).
#[tokio::test(flavor = "multi_thread")]
async fn perps_events_stream_empty_filter_matches_all() -> anyhow::Result<()> {
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
                let srv = actix_test::start(move || build_app_service(ctx.clone()));

                // Every set filter present but empty => all match-all.
                let res = srv
                    .get(format!(
                        "/perps/events/stream?since={since}&eventTypes=&pairIds=&users=&orderIds=&clientOrderIds="
                    ))
                    .send()
                    .await
                    .map_err(|err| anyhow!("request failed: {err}"))?;

                assert_eq!(res.status(), StatusCode::OK);

                let events = read_sse_events(res, 256).await?;
                let total: usize = events
                    .iter()
                    .filter_map(|(_, data)| {
                        serde_json::from_str::<
                            subscribe_perps_events2::SubscribePerpsEvents2PerpsEvents2,
                        >(data)
                        .ok()
                    })
                    .map(|batch| batch.events.len())
                    .sum();

                assert!(
                    total > 0,
                    "empty filters must match all events, not suppress them"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// A `since` older than the retained in-memory window fails the handshake with
/// `409 Conflict` (the `ResyncRequired` mapping) rather than opening a stream.
#[tokio::test(flavor = "multi_thread")]
async fn perps_events_stream_resync_required_is_409() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, ctx, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    // The first committed block is height 1, so the ring floor is >= 1 and
    // `floor - 1` is guaranteed to predate the window.
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
                let srv = actix_test::start(move || build_app_service(ctx.clone()));

                let res = srv
                    .get(format!("/perps/events/stream?since={stale}"))
                    .send()
                    .await
                    .map_err(|err| anyhow!("request failed: {err}"))?;

                assert_eq!(res.status(), StatusCode::CONFLICT);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
