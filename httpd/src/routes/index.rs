use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Responder, error::ErrorInternalServerError, get, web},
    async_graphql::futures_util::TryFutureExt,
    grug_types::GIT_COMMIT,
};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}

#[derive(serde::Serialize, Default)]
struct UpResponse<'a> {
    block_height: u64,
    git_commit: &'a str,
}

#[get("/up")]
pub async fn up(app_ctx: web::Data<Context>) -> Result<impl Responder, Error> {
    // This ensures that grug is working
    let block_height = app_ctx
        .grug_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)
        .await?
        .height;

    Ok(HttpResponse::Ok().json(UpResponse {
        block_height,
        git_commit: GIT_COMMIT,
    }))
}
