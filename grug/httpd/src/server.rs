use {
    super::error::Error,
    crate::{
        context::Context, middlewares::shutdown::ShutdownMiddleware, request_ip::access_logger,
        routes,
    },
    actix_cors::Cors,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::Compress,
        web::{self, ServiceConfig},
    },
    grug_types::HttpdConfig,
    sentry_actix::Sentry,
    std::sync::{Arc, atomic::AtomicBool},
};
#[cfg(feature = "metrics")]
use {
    actix_web_metrics::ActixWebMetricsBuilder,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    std::fmt::Display,
};

/// Run the HTTP server, includes GraphQL and REST endpoints.
/// The shutdown_flag should be set when signals are received to return 503 for new requests.
/// Actix Web handles graceful shutdown automatically on SIGTERM/SIGINT.
pub async fn run_server<CA, GS>(
    httpd_config: &HttpdConfig,
    context: Context,
    config_app: CA,
    build_schema: fn(Context) -> GS,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), Error>
where
    CA: Fn(Context, GS) -> Box<dyn Fn(&mut ServiceConfig)> + Clone + Send + 'static,
    GS: Clone + Send + 'static,
{
    let graphql_schema = build_schema(context.clone());

    #[cfg(feature = "tracing")]
    tracing::info!(httpd_config.ip, httpd_config.port, "Starting httpd server");

    #[cfg(feature = "metrics")]
    let metrics = ActixWebMetricsBuilder::new().build();

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
            .wrap(access_logger())
            .wrap(Compress::default())
            .wrap(cors);

        #[cfg(feature = "metrics")]
        let app = app.wrap(metrics.clone());

        app.configure(config_app(context.clone(), graphql_schema.clone()))
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

#[cfg(feature = "metrics")]
/// Run the metrics HTTP server
pub async fn run_metrics_server<I>(
    ip: I,
    port: u16,
    metrics_handler: PrometheusHandle,
) -> Result<(), Error>
where
    I: ToString + Display,
{
    #[cfg(feature = "tracing")]
    tracing::info!(%ip, port, "Starting metrics httpd server");

    let metrics = ActixWebMetricsBuilder::new().build();

    let recorder = PrometheusBuilder::new().build_recorder();
    let metrics_handler2 = recorder.handle();

    HttpServer::new(move || {
        let metrics_handler = metrics_handler.clone();
        let metrics_handler2 = metrics_handler2.clone();
        App::new()
            .wrap(metrics.clone())
            .wrap(Sentry::new())
            .wrap(access_logger())
            .wrap(Compress::default())
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().body("Metrics server is healthy") }),
            )
            .route(
                "/",
                web::get().to(|| async { HttpResponse::Ok().body("Metrics server is running") }),
            )
            .route(
                "/metrics",
                web::get().to(move || {
                    let metrics_handler = metrics_handler.clone();
                    let metrics_handler2 = metrics_handler2.clone();
                    metrics_handler2.run_upkeep();

                    async move {
                        let metrics2 = metrics_handler2.render();
                        let metrics = metrics_handler.render();
                        let combined = format!("{metrics}\n{metrics2}");

                        HttpResponse::Ok()
                            .content_type("text/plain; version=0.0.4")
                            .body(combined)
                    }
                }),
            )
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
            .service(routes::index::up)
            .service(routes::index::requester_ip)
            .service(routes::graphql::graphql_route::<
                crate::graphql::query::Query,
                async_graphql::EmptyMutation,
                async_graphql::EmptySubscription,
            >())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(app_ctx.graphql_rate_limiter.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
