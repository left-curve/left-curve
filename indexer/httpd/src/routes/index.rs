use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Responder, error::ErrorInternalServerError, get, web},
    async_graphql::futures_util::TryFutureExt,
    grug_app::GIT_COMMIT,
    indexer_sql::entity,
    sea_orm::{EntityTrait, Order, QueryOrder},
};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}

#[derive(serde::Serialize, Default)]
struct HealthResponse<'a> {
    block_height: u64,
    indexed_block_height: Option<u64>,
    git_commit: &'a str,
}

#[get("/up")]
pub async fn up(app_ctx: web::Data<Context>) -> Result<impl Responder, Error> {
    // This ensures than grug is working
    let block_height = app_ctx
        .grug_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)
        .await?
        .height;

    // This ensures than the database is up
    let indexed_block_height = entity::blocks::Entity::find()
        .order_by(entity::blocks::Column::BlockHeight, Order::Desc)
        .one(&app_ctx.db)
        .await
        .map_err(ErrorInternalServerError)?
        .map(|b| b.block_height as u64);

    Ok(HttpResponse::Ok().json(HealthResponse {
        block_height,
        indexed_block_height,
        git_commit: GIT_COMMIT,
    }))
}
