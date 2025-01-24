use {
    super::error::Error,
    crate::{context::Context, graphql::build_schema, routes},
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
        App, HttpResponse, HttpServer,
    },
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
pub async fn run_server(
    ip: Option<&str>,
    port: Option<u16>,
    database_url: String,
) -> Result<(), Error> {
    let port = port
        .or_else(|| {
            std::env::var("PORT")
                .ok()
                .and_then(|val| val.parse::<u16>().ok())
        })
        .unwrap_or(8080);
    let ip = ip.unwrap_or("0.0.0.0");

    let context = Context::new(Some(database_url)).await?;
    let graphql_schema = build_schema(context.clone());

    HttpServer::new(move || build_actix_app(context.clone(), graphql_schema.clone()))
        .bind((ip, port))?
        .run()
        .await?;

    Ok(())
}

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
    // S: async_graphql::ObjectType + Default + 'static,
    G: Clone + 'static,
{
    // merge_query!(MergedQuery, BlockQuery, MessageQuery, sub);

    // let graphql_schema = build_schema_with_sub::<S>(app_ctx.clone());
    // let graphql_schema = build_schema(app_ctx.clone());

    let app = App::new().wrap(Logger::default()).wrap(Compress::default());

    app.configure(config_app(app_ctx, graphql_schema))
}

pub fn config_app<G>(app_ctx: Context, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(routes::index::index)
            .service(routes::graphql::graphql_route())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
