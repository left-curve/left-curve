use {
    super::error::Error,
    crate::{
        context::FullContext,
        middlewares::shutdown::ShutdownMiddleware,
        routes::{
            self,
            graphql::{GraphqlRequestTimeout, graphql_route},
            index::index,
        },
        subscription_limiter::SubscriptionLimiter,
    },
    actix_cors::Cors,
    actix_files::Files,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
    },
    async_graphql::{EmptyMutation, EmptySubscription},
    grug_types::HttpdConfig,
    sentry_actix::Sentry,
    std::{
        sync::{Arc, atomic::AtomicBool, mpsc},
        time::Duration,
    },
};
#[cfg(feature = "metrics")]
use {crate::middlewares::metrics::init_httpd_metrics, actix_web_metrics::ActixWebMetricsBuilder};

pub fn config_app<G>(app_ctx: FullContext, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        let mut service_config = cfg
            .service(index)
            .service(routes::index::up)
            .service(routes::index::requester_ip)
            .service(routes::index::sentry_raise)
            .service(routes::blocks::services())
            .service(graphql_route::<
                crate::graphql::query::FullQuery,
                crate::graphql::mutation::IndexerMutation,
                crate::graphql::subscription::FullSubscription,
            >());

        // Add static file serving if static_files_path is configured
        if let Some(static_path) = &app_ctx.static_files_path {
            #[cfg(feature = "tracing")]
            tracing::info!(static_path, "Exposing static files at /static");

            service_config = service_config.service(
                Files::new("/static", static_path)
                    .prefer_utf8(true)
                    .use_last_modified(true),
            );
        }

        service_config
            .default_service(web::to(routes::index::not_found_handler))
            .app_data(web::Data::new(app_ctx.db.clone()))
            .app_data(web::Data::new(app_ctx.base.clone()))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}

/// Run the full-mode HTTP server (indexer features enabled).
///
/// The shutdown_flag should be set when signals are received to return 503 for
/// new requests. Actix Web handles graceful shutdown automatically on
/// SIGTERM/SIGINT.
///
/// If `port_sender` is provided, the actual bound port will be sent via the
/// channel after binding. Use port 0 to let the OS allocate an available port
/// (useful for tests).
pub async fn run_server(
    httpd_config: &HttpdConfig,
    context: FullContext,
    shutdown_flag: Arc<AtomicBool>,
    port_sender: Option<mpsc::Sender<u16>>,
) -> Result<(), Error> {
    let graphql_schema = crate::graphql::build_full_schema(context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(
        httpd_config.ip,
        httpd_config.port,
        "Starting indexer httpd server"
    );

    #[cfg(feature = "metrics")]
    let metrics = ActixWebMetricsBuilder::new().build();

    #[cfg(feature = "metrics")]
    init_httpd_metrics();

    let subscription_limiter = SubscriptionLimiter::new(
        httpd_config.max_subscriptions_per_connection,
        httpd_config.max_subscriptions_global,
    );

    let graphql_request_timeout = GraphqlRequestTimeout(Duration::from_secs(
        httpd_config.graphql_request_timeout_secs,
    ));

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

        app.app_data(web::Data::new(subscription_limiter.clone()))
            .app_data(web::Data::new(graphql_request_timeout))
            .configure(config_app(context.clone(), graphql_schema.clone()))
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

/// Run the chain-only HTTP server (no indexer features).
///
/// The `shutdown_flag` is wired into `ShutdownMiddleware` so the server returns
/// 503 for new requests once shutdown begins. Actix Web handles graceful
/// shutdown automatically on SIGTERM/SIGINT.
pub async fn run_minimal_server(
    httpd_config: &HttpdConfig,
    context: crate::context::MinimalContext,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), Error> {
    let graphql_schema = crate::graphql::minimal::build_minimal_schema(context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(
        httpd_config.ip,
        httpd_config.port,
        "Starting minimal httpd server"
    );

    #[cfg(feature = "metrics")]
    let metrics = ActixWebMetricsBuilder::new().build();

    #[cfg(feature = "metrics")]
    init_httpd_metrics();

    let subscription_limiter = SubscriptionLimiter::new(
        httpd_config.max_subscriptions_per_connection,
        httpd_config.max_subscriptions_global,
    );

    let graphql_request_timeout = GraphqlRequestTimeout(Duration::from_secs(
        httpd_config.graphql_request_timeout_secs,
    ));

    let cors_allowed_origin = httpd_config.cors_allowed_origin.clone();
    let shutdown_flag_clone = shutdown_flag.clone();
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
            .wrap(ShutdownMiddleware::new(shutdown_flag_clone.clone()))
            .wrap(Sentry::new())
            .wrap(Logger::default())
            .wrap(Compress::default())
            .wrap(cors);

        #[cfg(feature = "metrics")]
        let app = app.wrap(metrics.clone());

        app.app_data(web::Data::new(subscription_limiter.clone()))
            .app_data(web::Data::new(graphql_request_timeout))
            .service(crate::routes::index::index)
            .service(crate::routes::index::minimal_up)
            .service(crate::routes::index::requester_ip)
            .service(graphql_route::<
                crate::graphql::minimal::MinimalQuery,
                EmptyMutation,
                EmptySubscription,
            >())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(context.clone()))
            .app_data(web::Data::new(graphql_schema.clone()))
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
    .bind((&*httpd_config.ip, httpd_config.port))?
    .run()
    .await?;

    Ok(())
}
