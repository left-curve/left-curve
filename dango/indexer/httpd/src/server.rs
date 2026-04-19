use {
    crate::context::Context,
    actix_cors::Cors,
    actix_files::Files,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
    },
    grug_httpd::{
        middlewares::shutdown::ShutdownMiddleware,
        routes::{graphql::graphql_route, index::index},
    },
    grug_types::HttpdConfig,
    indexer_httpd::routes,
    sentry_actix::Sentry,
    std::sync::{Arc, atomic::AtomicBool, mpsc},
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
                indexer_httpd::graphql::mutation::IndexerMutation,
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

/// Run the dango HTTP server with dango-specific context.
/// The shutdown_flag should be set when signals are received to return 503 for new requests.
/// Actix Web handles graceful shutdown automatically on SIGTERM/SIGINT.
///
/// If `port_sender` is provided, the actual bound port will be sent via the channel after binding.
/// Use port 0 to let the OS allocate an available port (useful for tests).
pub async fn run_server(
    httpd_config: &HttpdConfig,
    dango_httpd_context: crate::context::Context,
    shutdown_flag: Arc<AtomicBool>,
    port_sender: Option<mpsc::Sender<u16>>,
) -> Result<(), indexer_httpd::error::Error> {
    let graphql_schema = crate::graphql::build_schema(dango_httpd_context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(
        httpd_config.ip,
        httpd_config.port,
        "Starting dango httpd server"
    );

    #[cfg(feature = "metrics")]
    let metrics = actix_web_metrics::ActixWebMetricsBuilder::new().build();

    #[cfg(feature = "metrics")]
    indexer_httpd::middlewares::metrics::init_httpd_metrics();

    let cors_allowed_origin = httpd_config.cors_allowed_origin.clone();
    let shutdown_flag_clone = shutdown_flag.clone();
    let server = HttpServer::new(move || {
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
            .wrap(ShutdownMiddleware::new(shutdown_flag_clone.clone()))
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
    .workers(httpd_config.workers)
    .max_connections(httpd_config.max_connections)
    .backlog(httpd_config.backlog)
    .keep_alive(actix_web::http::KeepAlive::Timeout(
        std::time::Duration::from_secs(httpd_config.keep_alive_secs),
    ))
    .client_request_timeout(std::time::Duration::from_secs(
        httpd_config.client_request_timeout_secs,
    ))
    .client_disconnect_timeout(std::time::Duration::from_secs(
        httpd_config.client_disconnect_timeout_secs,
    ))
    .worker_max_blocking_threads(httpd_config.worker_max_blocking_threads)
    .bind((&*httpd_config.ip, httpd_config.port))?;

    // Send the actual bound port if a channel was provided
    if let Some(sender) = port_sender
        && let Some(addr) = server.addrs().first()
    {
        let actual_port = addr.port();
        #[cfg(feature = "tracing")]
        tracing::info!(actual_port, "Server bound to port");
        let _ = sender.send(actual_port);
    }

    server.run().await?;

    Ok(())
}
