use {
    crate::{
        context::FullContext,
        subscription_limiter::{SubscriptionLimiter, guard_subscription_stream},
    },
    actix_web::{
        Error, HttpRequest, HttpResponse, Scope,
        error::{ErrorConflict, ErrorTooManyRequests},
        get, web,
    },
    actix_web_lab::sse,
    dango_indexer_stream::make_perps_filter,
    futures_util::StreamExt,
    serde::Deserialize,
    std::{collections::HashSet, sync::Arc},
};

pub fn services() -> Scope {
    web::scope("/perps").service(perps_events_stream)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PerpsStreamQuery {
    since: Option<u64>,
    event_types: Option<String>,
    pair_ids: Option<String>,
    users: Option<String>,
    order_ids: Option<String>,
    client_order_ids: Option<String>,
}

/// Parse a comma-separated filter parameter into a set of values.
///
/// An absent parameter (`None`) and an empty one (`Some("")`, or only
/// separators) both collapse to `None` — i.e. "do not filter on this field",
/// which [`make_perps_filter`] treats as match-all. Empty segments are dropped,
/// so a stray trailing comma (`"a,"`) yields `{a}`.
fn parse_set(raw: Option<String>) -> Option<HashSet<String>> {
    let set: HashSet<String> = raw?
        .split(',')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect();

    (!set.is_empty()).then_some(set)
}

/// Stream perps-exchange contract events (e.g. `order_filled`, `liquidated`,
/// `deleveraged`, `order_persisted`, `order_removed`) in real time as
/// Server-Sent Events, grouped per block — the same `PerpsEvent2Batch` shape the
/// `perps_events2` GraphQL subscription emits, one event per block that has at
/// least one matching contract event. Each event's `id` is the block height, so
/// a client reconnecting with a `Last-Event-ID` header resumes after the last
/// block it received.
///
/// `eventTypes`, `pairIds`, `users`, `orderIds`, and `clientOrderIds` are
/// comma-separated sets that AND together; an absent or empty parameter does not
/// filter on that field. Each value is matched verbatim against the event's
/// canonical string form (address, denom, or decimal id). `?since=<height>`
/// replays the retained in-memory window from that height (inclusive) before the
/// live tail; a `since` that predates the window fails with `409 Conflict` —
/// reconnect with a newer height (deep history is on the `perpsEvents` query).
/// The stream is capped by the shared subscription limiter and returns
/// `429 Too Many Requests` when the global limit is reached.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/events/stream")]
pub async fn perps_events_stream(
    req: HttpRequest,
    query: web::Query<PerpsStreamQuery>,
    app_ctx: web::Data<FullContext>,
    limiter: web::Data<SubscriptionLimiter>,
) -> Result<HttpResponse, Error> {
    let guard = Arc::new(
        limiter
            .new_connection()
            .try_acquire()
            .map_err(|err| ErrorTooManyRequests(err.message))?,
    );

    let query = query.into_inner();
    let since = crate::routes::resolve_since(&req, query.since);

    let filter = make_perps_filter(
        parse_set(query.event_types),
        parse_set(query.pair_ids),
        parse_set(query.users),
        parse_set(query.order_ids),
        parse_set(query.client_order_ids),
    );

    let stream = app_ctx
        .stream_context
        .perps()
        .subscribe(since, filter)
        .map_err(|resync| ErrorConflict(resync.to_string()))?;

    let events = guard_subscription_stream(stream, Some(guard)).map(|block| {
        let id = block.block_height.to_string();
        sse::Data::new_json(&block).map(|data| sse::Event::Data(data.id(id)))
    });

    Ok(crate::routes::sse_response(&req, events))
}
