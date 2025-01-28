use {
    super::error::Error,
    crate::{context::Context, routes},
    actix_web::{
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
        App, HttpResponse, HttpServer,
    },
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
pub async fn run_server<CA, GS>(
    ip: Option<&str>,
    port: Option<u16>,
    database_url: String,
    config_app: CA,
    build_schema: fn(Context) -> GS,
) -> Result<(), Error>
where
    CA: Fn(Context, GS) -> Box<dyn Fn(&mut ServiceConfig)> + Clone + Send + 'static,
    GS: Clone + Send + 'static,
{
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

    HttpServer::new(move || {
        let app = App::new().wrap(Logger::default()).wrap(Compress::default());

        app.configure(config_app(context.clone(), graphql_schema.clone()))
    })
    .bind((ip, port))?
    .run()
    .await?;

    Ok(())
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
