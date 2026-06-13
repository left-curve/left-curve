use {
    crate::{
        context::{FullContext, MinimalContext},
        request_ip::RequesterIp,
    },
    actix_web::{
        Error, HttpRequest, HttpResponse, Responder, error::ErrorInternalServerError, get, web,
    },
    async_graphql::futures_util::TryFutureExt,
    chrono::{Duration, Utc},
    dango_indexer_sql::entity,
    dango_primitives::{BlockInfo, GIT_COMMIT},
    sea_orm::{EntityTrait, Order, QueryOrder},
    std::env::var,
};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}

#[get("/requester-ip")]
pub async fn requester_ip(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json(RequesterIp::from_request(&req))
}

#[derive(serde::Serialize)]
pub struct UpResponse<'a> {
    pub block: BlockInfo,
    pub is_running: bool,
    pub git_commit: &'a str,
    pub indexed_block_height: Option<u64>,
    pub chain_id: &'a str,
    pub hostname: &'a str,
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/up")]
pub async fn up(app_ctx: web::Data<FullContext>) -> Result<impl Responder, Error> {
    // This ensures that grug is working
    let block = app_ctx
        .base
        .dango_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)
        .await?;

    let is_running =
        block.timestamp.to_naive_date_time() >= (Utc::now().naive_utc() - Duration::seconds(30));

    // This ensures than the database is up
    let indexed_block_height = entity::blocks::Entity::find()
        .order_by(entity::blocks::Column::BlockHeight, Order::Desc)
        .one(&app_ctx.db)
        .await
        .map_err(ErrorInternalServerError)?
        .map(|b| b.block_height as u64);

    Ok(HttpResponse::Ok().json(UpResponse {
        block,
        is_running,
        indexed_block_height,
        git_commit: GIT_COMMIT,
        chain_id: var("CHAIN_ID").unwrap_or_default().as_str(),
        hostname: var("HOSTNAME").unwrap_or_default().as_str(),
    }))
}

/// Chain-only variant of `/up`. Returns the same JSON shape as
/// [`up`], with `indexed_block_height: None` because chain-only mode has
/// no Postgres to query.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/up")]
pub async fn minimal_up(app_ctx: web::Data<MinimalContext>) -> Result<impl Responder, Error> {
    let block = app_ctx
        .dango_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)
        .await?;

    let is_running =
        block.timestamp.to_naive_date_time() >= (Utc::now().naive_utc() - Duration::seconds(30));

    Ok(HttpResponse::Ok().json(UpResponse {
        block,
        is_running,
        indexed_block_height: None,
        git_commit: GIT_COMMIT,
        chain_id: var("CHAIN_ID").unwrap_or_default().as_str(),
        hostname: var("HOSTNAME").unwrap_or_default().as_str(),
    }))
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[get("/sentry-raise")]
pub async fn sentry_raise() -> Result<impl Responder, Error> {
    sentry::capture_message("Capturing a message before a crash", sentry::Level::Info);

    let err = "NaN".parse::<usize>().unwrap_err();
    sentry::capture_error(&err);

    Ok(HttpResponse::Ok().body("Sending a sentry crash"))
}

/// Custom 404 handler that serves an HTML page from
/// `{static_files_path}/404.html` when configured, falling back to a plain
/// text response otherwise.
pub async fn not_found_handler(app_ctx: web::Data<FullContext>) -> HttpResponse {
    let static_files_path = app_ctx.static_files_path.as_deref();

    if let Some(static_files_path) = static_files_path {
        let file_path = format!("{static_files_path}/404.html");
        if let Ok(html_content) = std::fs::read_to_string(&file_path) {
            return HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(html_content);
        }
    }

    HttpResponse::NotFound()
        .content_type("text/plain; charset=utf-8")
        .body("404 Not Found")
}
