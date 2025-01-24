use {
    crate::routes::graphql,
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
        App, HttpResponse, HttpServer,
    },
    async_graphql::MergedObject,
    indexer_httpd::{context::Context, routes::index},
};

pub fn build_actix_app<G>(
    app_ctx: Context,
    graphql_schema: G,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
>
where
    G: Clone + 'static,
{
    let app = App::new().wrap(Logger::default()).wrap(Compress::default());

    app.configure(config_app(app_ctx, graphql_schema))
}

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
