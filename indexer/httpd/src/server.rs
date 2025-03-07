use {
    super::error::Error,
    crate::{context::Context, routes},
    actix_cors::Cors,
    actix_web::{
        http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
        App, HttpResponse, HttpServer,
    },
    std::fmt::Display,
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
pub async fn run_server<CA, GS, I>(
    ip: I,
    port: u16,
    cors_allowed_origin: Option<String>,
    context: Context,
    config_app: CA,
    build_schema: fn(Context) -> GS,
) -> Result<(), Error>
where
    CA: Fn(Context, GS) -> Box<dyn Fn(&mut ServiceConfig)> + Clone + Send + 'static,
    GS: Clone + Send + 'static,
    I: ToString + Display,
{
    let graphql_schema = build_schema(context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!("Starting indexer httpd server at {ip}:{port}");

    HttpServer::new(move || {
        let cors = if let Some(origin) = cors_allowed_origin.as_deref() {
            Cors::default()
                .allowed_origin(origin)
                .allowed_methods(vec!["POST"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                .allowed_header(http::header::CONTENT_TYPE)
                .max_age(3600)
        } else {
            Cors::default()
        };

        let app = App::new()
            .wrap(Logger::default())
            .wrap(Compress::default())
            .wrap(cors);

        app.configure(config_app(context.clone(), graphql_schema.clone()))
    })
    .bind((ip.to_string(), port))?
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
