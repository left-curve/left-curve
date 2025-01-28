use {
    crate::{graphql::build_schema, routes::graphql},
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web::{self, ServiceConfig},
        App, HttpResponse,
    },
    indexer_httpd::{context::Context, routes::index, server::build_actix_app_with_config},
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
// pub async fn run_server(
//     ip: Option<&str>,
//     port: Option<u16>,
//     database_url: String,
// ) -> Result<(), indexer_httpd::error::Error> {
//     let port = port
//         .or_else(|| {
//             std::env::var("PORT")
//                 .ok()
//                 .and_then(|val| val.parse::<u16>().ok())
//         })
//         .unwrap_or(8080);
//     let ip = ip.unwrap_or("0.0.0.0");

//     let context = Context::new(Some(database_url)).await?;

//     HttpServer::new(move || build_actix_app(context.clone()))
//         .bind((ip, port))?
//         .run()
//         .await?;

//     Ok(())
// }

pub fn build_actix_app(
    app_ctx: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(app_ctx.clone());

    build_actix_app_with_config(app_ctx, graphql_schema, |app_ctx, graphql_schema| {
        config_app(app_ctx, graphql_schema)
    })
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
