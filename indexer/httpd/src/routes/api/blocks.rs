use {
    crate::context::Context,
    actix_web::{get, web, Error, HttpResponse},
    indexer_sql::block_to_index::BlockToIndex,
};

#[get("/api/blocks/{block_height}")]
pub async fn block_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    let block_height: u64 = path.into_inner();

    let block_filename = app_ctx.indexer_path.block_path(block_height);

    if !BlockToIndex::exists(block_filename.clone()) {
        println!("Block not found: {:?}", block_filename);
        return Ok(HttpResponse::NotFound().body("Block not found"));
    }

    match BlockToIndex::load_from_disk(block_filename) {
        Ok(data) => Ok(HttpResponse::Ok().json(data.block)),
        Err(_err) => Ok(HttpResponse::InternalServerError().body("Failed to load block file")),
    }
}

#[get("/api/block_results/{block_height}")]
pub async fn block_results_by_height(
    path: web::Path<u64>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    let block_height: u64 = path.into_inner();

    let block_filename = app_ctx.indexer_path.block_path(block_height);

    match BlockToIndex::load_from_disk(block_filename) {
        Ok(data) => Ok(HttpResponse::Ok().json(data.block_outcome)),
        Err(_err) => Ok(HttpResponse::InternalServerError().body("Failed to load block file")),
    }
}
