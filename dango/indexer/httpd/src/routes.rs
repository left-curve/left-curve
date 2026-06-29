pub mod blocks;
pub mod graphql;
pub mod index;
pub mod perps;

use {
    actix_web::{
        HttpRequest, HttpResponse, Responder,
        http::header::{CONTENT_ENCODING, HeaderValue},
    },
    actix_web_lab::sse,
    futures_util::Stream,
    std::time::Duration,
};

/// Keep-alive comment cadence for the SSE subscription feeds. The periodic
/// comment keeps an idle connection (e.g. a narrowly-filtered perps feed that
/// matches no events for a while) alive through intermediaries, and surfaces a
/// dead socket promptly so its subscription slot is released.
const SSE_KEEP_ALIVE_SECS: u64 = 15;

/// Resolve the block height a stream subscription should start from.
///
/// An explicit `?since=<height>` query parameter wins and is inclusive
/// (matching the `sinceBlockHeight` GraphQL argument). Otherwise a reconnecting
/// client sends the `Last-Event-ID` header carrying the height of the last block
/// it received; we resume at the next height so that block is not re-delivered.
/// Absent both, the stream starts from the live tip.
pub(crate) fn resolve_since(req: &HttpRequest, explicit: Option<u64>) -> Option<u64> {
    if let Some(height) = explicit {
        return Some(height);
    }

    req.headers()
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.parse::<u64>().ok())
        .map(|height| height.saturating_add(1))
}

/// Wrap a stream of pre-built SSE events into a `text/event-stream` response.
///
/// Sets `Content-Encoding: identity` so the app-wide `Compress` middleware
/// leaves the body untouched: it treats `text/event-stream` as compressible and
/// would otherwise buffer the (unbounded) stream waiting to compress it, so
/// nothing would ever reach the client. The encoder skips any response that
/// already declares an encoding, which is exactly what `identity` does here.
pub(crate) fn sse_response<S, E>(req: &HttpRequest, events: S) -> HttpResponse
where
    S: Stream<Item = Result<sse::Event, E>> + 'static,
    E: Into<Box<dyn std::error::Error>> + 'static,
{
    let body =
        sse::Sse::from_stream(events).with_keep_alive(Duration::from_secs(SSE_KEEP_ALIVE_SECS));

    let mut res = body.respond_to(req);
    res.headers_mut()
        .insert(CONTENT_ENCODING, HeaderValue::from_static("identity"));

    res
}
