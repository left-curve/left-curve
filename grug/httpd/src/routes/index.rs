use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Responder, error::ErrorInternalServerError, get, web},
    async_graphql::futures_util::TryFutureExt,
    chrono::{Duration, Utc},
    grug_types::{BlockInfo, GIT_COMMIT},
};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}

#[derive(serde::Serialize)]
struct UpResponse<'a> {
    block: BlockInfo,
    is_running: bool,
    git_commit: &'a str,
}

#[get("/up")]
pub async fn up(app_ctx: web::Data<Context>) -> Result<impl Responder, Error> {
    // This ensures that grug is working
    let block = app_ctx
        .grug_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)
        .await?;

    let is_running =
        block.timestamp.to_naive_date_time() >= (Utc::now().naive_utc() - Duration::seconds(30));

    Ok(HttpResponse::Ok().json(UpResponse {
        block,
        is_running,
        git_commit: GIT_COMMIT,
    }))
}
