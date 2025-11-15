use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Scope, error::ErrorInternalServerError, get, web},
    indexer_cache::cache_file::CacheFile,
    std::path::PathBuf,
};

pub fn services() -> Scope {
    web::scope("/block")
        .service(block_info_by_height)
        .service(latest_block_info)
        .service(block_result_by_height)
        .service(block_result)
}

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

fn check_block_exists(block_filename: PathBuf, height: u64) -> Result<(), Error> {
    if !CacheFile::exists(block_filename) {
        Err(actix_web::error::ErrorNotFound(format!(
            "block not found: {height}",
        )))
    } else {
        Ok(())
    }
}
