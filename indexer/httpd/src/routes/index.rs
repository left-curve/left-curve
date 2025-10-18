use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Responder, error::ErrorInternalServerError, get, web},
    async_graphql::futures_util::TryFutureExt,
    chrono::{Duration, Utc},
    grug_httpd::routes::index::UpResponse,
    grug_types::GIT_COMMIT,
    indexer_sql::entity,
    sea_orm::{EntityTrait, Order, QueryOrder},
};

#[get("/up")]
pub async fn up(app_ctx: web::Data<Context>) -> Result<impl Responder, Error> {
    // This ensures that grug is working
    let block = app_ctx
        .base
        .grug_app
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
    }))
}

#[get("/sentry-raise")]
pub async fn sentry_raise() -> Result<impl Responder, Error> {
    sentry::capture_message("Capturing a message before a crash", sentry::Level::Info);

    let err = "NaN".parse::<usize>().unwrap_err();
    sentry::capture_error(&err);

    Ok(HttpResponse::Ok().body("Sending a sentry crash"))
}
