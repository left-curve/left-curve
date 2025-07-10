use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, error::ErrorInternalServerError, get, web},
    indexer_sql::block_to_index::BlockToIndex,
    std::path::PathBuf,
};

#[get("/info")]
pub async fn latest_block_info(app_ctx: web::Data<Context>) -> Result<HttpResponse, Error> {
    let block_height = app_ctx
        .grug_app()
        .last_finalized_block()
        .await
        .map_err(ErrorInternalServerError)?
        .height;

    _block_by_height(block_height, &app_ctx)
}

#[get("/info/{block_height}")]
pub async fn block_info_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    _block_by_height(path.into_inner(), &app_ctx)
}

fn _block_by_height(block_height: u64, app_ctx: &Context) -> Result<HttpResponse, Error> {
    let block_filename = app_ctx.indexer_path.block_path(block_height);

    check_block_exists(block_filename.clone(), block_height)?;

    match BlockToIndex::load_from_disk(block_filename) {
        Ok(data) => Ok(HttpResponse::Ok().json(data.block)),
        Err(_err) => Ok(HttpResponse::InternalServerError().body("Failed to load block file")),
    }
}

#[get("/result")]
pub async fn block_result(app_ctx: web::Data<Context>) -> Result<HttpResponse, Error> {
    let block_height = app_ctx
        .grug_app()
        .last_finalized_block()
        .await
        .map_err(ErrorInternalServerError)?
        .height;

    _block_results_by_height(block_height, &app_ctx)
}

#[get("/result/{block_height}")]
pub async fn block_result_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    _block_results_by_height(path.into_inner(), &app_ctx)
}

fn _block_results_by_height(block_height: u64, app_ctx: &Context) -> Result<HttpResponse, Error> {
    let block_filename = app_ctx.indexer_path.block_path(block_height);

    check_block_exists(block_filename.clone(), block_height)?;

    match BlockToIndex::load_from_disk(block_filename) {
        Ok(data) => Ok(HttpResponse::Ok().json(data.block_outcome)),
        Err(_err) => Ok(HttpResponse::InternalServerError().body("Failed to load block file")),
    }
}

fn check_block_exists(block_filename: PathBuf, height: u64) -> Result<(), Error> {
    if !BlockToIndex::exists(block_filename) {
        Err(actix_web::error::ErrorNotFound(format!(
            "block not found: {height}",
        )))
    } else {
        Ok(())
    }
}
