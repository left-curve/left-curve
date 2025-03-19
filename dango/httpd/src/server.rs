use {
    crate::routes::graphql,
    actix_web::{
        web::{self, ServiceConfig},
        HttpResponse,
    },
    indexer_httpd::{context::Context, routes},
};

pub fn config_app<G>(app_ctx: Context, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(routes::index::index)
            .service(routes::api::blocks::block_by_height)
            .service(routes::api::blocks::block_results_by_height)
            .service(graphql::graphql_route())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
