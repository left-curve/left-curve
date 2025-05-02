use {
    crate::context::Context,
    actix_web::{Error, HttpResponse, Responder, error::ErrorInternalServerError, get, web},
    indexer_sql::entity,
    sea_orm::{EntityTrait, Order, QueryOrder},
};

#[get("/")]
pub async fn index() -> impl Responder {
    "OK"
}

#[derive(serde::Serialize, Default)]
struct HealthResponse {
    block_height: u64,
    indexed_block_height: Option<u64>,
}

#[get("/up")]
pub async fn up(app_ctx: web::Data<Context>) -> Result<impl Responder, Error> {
    // This ensures than grug is working
    let block_height = app_ctx
        .grug_app
        .last_finalized_block()
        .map_err(ErrorInternalServerError)?
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
    }))
}

// #[cfg(test)]
// mod tests {
//     use actix_web::http::StatusCode;
//     use actix_web::test;
//
//     #[actix_web::test]
//     async fn returns_200() -> Result<(), anyhow::Error> {
//         let app = test::init_service(create_test_app(test_ctx).await).await;
//
//         let req = test::TestRequest::get().uri("/up").to_request();
//
//         let resp = test::call_service(&app, req).await;
//         assert_eq!(resp.status(), StatusCode::OK);
//
//         Ok(())
//     }
// }
