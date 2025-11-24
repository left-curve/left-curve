use {
    crate::context::Context,
    actix_cors::Cors,
    actix_files::Files,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
    },
    grug_httpd::routes::{graphql::graphql_route, index::index},
    indexer_httpd::routes,
    sentry_actix::Sentry,
};

/// Custom 404 handler that serves a nice HTML page
async fn not_found_handler(app_ctx: web::Data<Context>) -> HttpResponse {
    let static_files_path = app_ctx.static_files_path.as_deref();

    if let Some(static_files_path) = static_files_path {
        let file_path = format!("{static_files_path}/404.html");
        if let Ok(html_content) = std::fs::read_to_string(&file_path) {
            return HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(html_content);
        }
    }

    HttpResponse::NotFound()
        .content_type("text/plain; charset=utf-8")
        .body("404 Not Found")
}

pub fn config_app<G>(
    dango_httpd_context: Context,
    graphql_schema: G,
) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        let mut service_config = cfg
            .service(index)
            .service(routes::index::up)
            .service(routes::index::sentry_raise)
            .service(routes::blocks::services())
            .service(graphql_route::<
                crate::graphql::query::Query,
                indexer_httpd::graphql::mutation::Mutation,
                crate::graphql::subscription::Subscription,
            >());

        // Add static file serving if static_files_path is configured
        if let Some(static_path) = &dango_httpd_context.static_files_path {
            #[cfg(feature = "tracing")]
            tracing::info!(static_path, "Exposing static files at /static");

            service_config = service_config.service(
                Files::new("/static", static_path)
                    .prefer_utf8(true)
                    .use_last_modified(true),
            );
        }

        service_config
            .default_service(web::to(not_found_handler))
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
    let graphql_schema = crate::graphql::build_schema(dango_httpd_context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(%ip, port, "Starting dango httpd server");

    #[cfg(feature = "metrics")]
    let metrics = actix_web_metrics::ActixWebMetricsBuilder::new().build();

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
            for origin in origin.split(',') {
                cors = cors.allowed_origin(origin.trim());
            }
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
    .workers(8)
    .max_connections(10_000)
    .backlog(8192)
    .keep_alive(actix_web::http::KeepAlive::Os)
    .worker_max_blocking_threads(16)
    .bind((ip.to_string(), port))?
    .run()
    .await?;

    Ok(())
}
