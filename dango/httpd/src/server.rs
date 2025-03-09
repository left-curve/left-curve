use {
    crate::routes::graphql,
    actix_web::{
        HttpResponse,
        web::{self, ServiceConfig},
    },
    indexer_httpd::{context::Context, routes::index},
};

pub fn config_app<G>(app_ctx: Context, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(index::index)
            .service(graphql::graphql_route())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
