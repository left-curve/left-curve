use {
    crate::{
        context::FullContext,
        subscription_limiter::{SubscriptionLimiter, guard_subscription_stream},
    },
    actix_web::{
        Error, HttpRequest, HttpResponse, Scope,
        error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorTooManyRequests},
        get, web,
    },
    actix_web_lab::sse,
    dango_indexer_cache::cache_file::CacheFile,
    dango_primitives::FullBlock,
    futures_util::StreamExt,
    serde::Deserialize,
    std::{path::PathBuf, sync::Arc},
};

/// Maximum number of blocks returned by `/block/full/range` in one request.
const MAX_FULL_BLOCK_RANGE: u64 = 20;

pub fn services() -> Scope {
    web::scope("/block")
        .service(block_info_by_height)
        .service(latest_block_info)
        .service(block_result_by_height)
        .service(block_result)
        // `/full/range` and `/full/stream` are registered before
        // `/full/{block_height}` so their literal segments are not captured as a
        // block height.
        .service(full_block_range)
        .service(full_block_stream)
        .service(full_block_by_height)
        .service(latest_full_block)
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/info")]
pub async fn latest_block_info(app_ctx: web::Data<FullContext>) -> Result<HttpResponse, Error> {
    let block_height = app_ctx
        .dango_app()
        .last_finalized_block()
        .await
        .map_err(ErrorInternalServerError)?
        .height;

    _block_by_height(block_height, &app_ctx)
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/info/{block_height}")]
pub async fn block_info_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<FullContext>,
) -> Result<HttpResponse, Error> {
    _block_by_height(path.into_inner(), &app_ctx)
}

fn _block_by_height(block_height: u64, app_ctx: &FullContext) -> Result<HttpResponse, Error> {
    let block_filename = app_ctx
        .indexer_cache_context
        .indexer_path
        .block_path(block_height);

    check_block_exists(block_filename.clone(), block_height)?;

    match CacheFile::load_from_disk(block_filename) {
        Ok(cache_file) => Ok(HttpResponse::Ok().json(cache_file.data.block)),
        Err(err) => {
            Ok(HttpResponse::InternalServerError()
                .body(format!("failed to load block file: {err}")))
        },
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/result")]
pub async fn block_result(app_ctx: web::Data<FullContext>) -> Result<HttpResponse, Error> {
    let block_height = app_ctx
        .dango_app()
        .last_finalized_block()
        .await
        .map_err(ErrorInternalServerError)?
        .height;

    _block_results_by_height(block_height, &app_ctx)
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/result/{block_height}")]
pub async fn block_result_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<FullContext>,
) -> Result<HttpResponse, Error> {
    _block_results_by_height(path.into_inner(), &app_ctx)
}

fn _block_results_by_height(
    block_height: u64,
    app_ctx: &FullContext,
) -> Result<HttpResponse, Error> {
    let block_filename = app_ctx
        .indexer_cache_context
        .indexer_path
        .block_path(block_height);

    check_block_exists(block_filename.clone(), block_height)?;

    match CacheFile::load_from_disk(block_filename) {
        Ok(cache_file) => Ok(HttpResponse::Ok().json(cache_file.data.block_outcome)),
        Err(err) => {
            Ok(HttpResponse::InternalServerError()
                .body(format!("failed to load block file: {err}")))
        },
    }
}

// ---- /full: block info + outcome together ----

/// The latest finalized block — both its info and its outcome.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/full")]
pub async fn latest_full_block(app_ctx: web::Data<FullContext>) -> Result<HttpResponse, Error> {
    let block_height = app_ctx
        .dango_app()
        .last_finalized_block()
        .await
        .map_err(ErrorInternalServerError)?
        .height;

    _full_block_by_height(block_height, &app_ctx)
}

/// A specific block by height — both its info and its outcome.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/full/{block_height}")]
pub async fn full_block_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<FullContext>,
) -> Result<HttpResponse, Error> {
    _full_block_by_height(path.into_inner(), &app_ctx)
}

/// A contiguous chunk of full blocks starting at `from`, through `to`
/// (inclusive), capped at [`MAX_FULL_BLOCK_RANGE`]. Iteration stops at the first
/// height with no block on disk, so the result is always a gap-free run from
/// `from`: a range past the chain tip returns the blocks up to the tip, and if
/// `from` itself is missing the result is empty.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/full/range")]
pub async fn full_block_range(
    query: web::Query<RangeQuery>,
    app_ctx: web::Data<FullContext>,
) -> Result<HttpResponse, Error> {
    let RangeQuery { from, to } = query.into_inner();

    if from > to {
        return Err(ErrorBadRequest(format!(
            "invalid range: from ({from}) must be <= to ({to})"
        )));
    }

    let to = to.min(from.saturating_add(MAX_FULL_BLOCK_RANGE - 1));

    let mut blocks = Vec::with_capacity((to - from + 1) as usize);
    for block_height in from..=to {
        let block_filename = app_ctx
            .indexer_cache_context
            .indexer_path
            .block_path(block_height);

        if CacheFile::exists(block_filename.clone()) {
            blocks.push(load_full_block(block_filename)?);
        } else {
            break;
        }
    }

    Ok(HttpResponse::Ok().json(blocks))
}

// ---- /full/stream: live SSE feed of full blocks ----

#[derive(Deserialize)]
struct StreamQuery {
    since: Option<u64>,
}

/// Stream every finalized block in real time as Server-Sent Events — the same
/// `FullBlock` shape (`block` + `outcome`) the `/full/{block_height}` route
/// returns, one event per block. Each event's `id` is the block height, so a
/// client reconnecting with a `Last-Event-ID` header resumes after the last
/// block it received.
///
/// The feed is served from the validator's in-memory window. `?since=<height>`
/// replays the retained blocks from that height (inclusive) before streaming the
/// live tail; without it, only blocks newer than the current tip are streamed.
/// A `since` that predates the retained window fails with `409 Conflict` —
/// reconnect with a newer height (deep history is on the `/full/{block_height}`
/// and `/full/range` routes). The stream is capped by the shared subscription
/// limiter and returns `429 Too Many Requests` when the global limit is reached.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/full/stream")]
pub async fn full_block_stream(
    req: HttpRequest,
    query: web::Query<StreamQuery>,
    app_ctx: web::Data<FullContext>,
    limiter: web::Data<SubscriptionLimiter>,
) -> Result<HttpResponse, Error> {
    let guard = Arc::new(
        limiter
            .new_connection()
            .try_acquire()
            .map_err(|err| ErrorTooManyRequests(err.message))?,
    );

    let since = crate::routes::resolve_since(&req, query.since);

    // Every block matches (no filtering); clone the block as-is.
    let stream = app_ctx
        .stream_context
        .blocks()
        .subscribe(since, |block: &FullBlock| Some(block.clone()))
        .map_err(|resync| ErrorConflict(resync.to_string()))?;

    let events = guard_subscription_stream(stream, Some(guard)).map(|block| {
        let id = block.block.info.height.to_string();
        sse::Data::new_json(&block).map(|data| sse::Event::Data(data.id(id)))
    });

    Ok(crate::routes::sse_response(&req, events))
}

fn _full_block_by_height(block_height: u64, app_ctx: &FullContext) -> Result<HttpResponse, Error> {
    let block_filename = app_ctx
        .indexer_cache_context
        .indexer_path
        .block_path(block_height);

    check_block_exists(block_filename.clone(), block_height)?;

    Ok(HttpResponse::Ok().json(load_full_block(block_filename)?))
}

/// Load a block file from disk and project it to `block` + `block_outcome`,
/// dropping the `http_request_details` (client IPs) the file also holds — the
/// same projection the `/info` and `/result` routes use. Returns the same
/// `FullBlock` shape as the `/full/stream` SSE feed.
fn load_full_block(block_filename: PathBuf) -> Result<FullBlock, Error> {
    let cache_file = CacheFile::load_from_disk(block_filename)
        .map_err(|err| ErrorInternalServerError(format!("failed to load block file: {err}")))?;

    Ok(FullBlock {
        block: cache_file.data.block,
        outcome: cache_file.data.block_outcome,
    })
}

#[derive(Deserialize)]
struct RangeQuery {
    from: u64,
    to: u64,
}

fn check_block_exists(block_filename: PathBuf, height: u64) -> Result<(), Error> {
    if !CacheFile::exists(block_filename) {
        Err(actix_web::error::ErrorNotFound(format!(
            "block not found: {height}",
        )))
    } else {
        Ok(())
    }
}
