use {
    crate::{context::Context, routes::graphql},
    actix_web::{
        HttpResponse,
        web::{self, ServiceConfig},
    },
    indexer_httpd::routes,
};

pub fn config_app<G>(
    dango_httpd_context: Context,
    graphql_schema: G,
) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(routes::index::index)
            .service(routes::index::up)
            .service(routes::api::services::api_services())
            .service(graphql::graphql_route())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(dango_httpd_context.db.clone()))
            .app_data(web::Data::new(dango_httpd_context.clone()))
            .app_data(web::Data::new(
                dango_httpd_context.indexer_httpd_context.clone(),
            ))
            .app_data(web::Data::new(
                dango_httpd_context.indexer_httpd_context.base.clone(),
            ))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}

/// Run the dango HTTP server with dango-specific context
pub async fn run_server<I>(
    ip: I,
    port: u16,
    cors_allowed_origin: Option<String>,
    dango_httpd_context: crate::context::Context,
) -> Result<(), indexer_httpd::error::Error>
where
    I: ToString + std::fmt::Display,
{
    use {
        actix_cors::Cors,
        actix_web::{
            App, HttpServer, http,
            middleware::{Compress, Logger},
        },
        sentry_actix::Sentry,
    };

    let graphql_schema = crate::graphql::build_schema(dango_httpd_context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(%ip, port, "Starting dango httpd server");

    #[cfg(feature = "metrics")]
    let metrics = actix_web_metrics::ActixWebMetricsBuilder::new()
        .build()
        .unwrap();

    #[cfg(feature = "metrics")]
    indexer_httpd::middlewares::metrics::init_httpd_metrics();

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["POST", "GET", "OPTIONS"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_static("sentry-trace"),
                http::header::HeaderName::from_static("baggage"),
            ])
            .max_age(3600);

        if let Some(origin) = cors_allowed_origin.as_deref() {
            cors = cors.allowed_origin(origin);
        } else {
            cors = cors.allow_any_origin();
        }

        let app = App::new()
            .wrap(Sentry::new())
            .wrap(Logger::default())
            .wrap(Compress::default())
            .wrap(cors);

        #[cfg(feature = "metrics")]
        let app = app.wrap(metrics.clone());

        app.configure(config_app(
            dango_httpd_context.clone(),
            graphql_schema.clone(),
        ))
    })
    .bind((ip.to_string(), port))?
    .run()
    .await?;

    Ok(())
}
