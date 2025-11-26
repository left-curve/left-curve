use {
    super::error::Error,
    actix_cors::Cors,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self},
    },
    sentry_actix::Sentry,
    std::fmt::Display,
};
#[cfg(feature = "metrics")]
use {actix_web_metrics::ActixWebMetricsBuilder, metrics_exporter_prometheus::PrometheusHandle};

/// Run the bridge relayer HTTP server
pub async fn run_server<I>(
    ip: I,
    port: u16,
    cors_allowed_origin: Option<String>,
) -> Result<(), Error>
where
    I: ToString + std::fmt::Display,
{
    #[cfg(feature = "tracing")]
    tracing::info!(%ip, port, "Starting bridge relayer httpd server");

    #[cfg(feature = "metrics")]
    let metrics = actix_web_metrics::ActixWebMetricsBuilder::new()
        .build()
        .unwrap();

    #[cfg(feature = "metrics")]
    crate::middlewares::metrics::init_httpd_metrics();

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

        app.route(
            "/health",
            web::get().to(|| async { HttpResponse::Ok().body("Bridge relayer server is healthy") }),
        )
        .service(crate::routes::deposit_address)
    })
    .workers(8)
    // .max_connections(10_000)
    // .backlog(8192)
    .keep_alive(actix_web::http::KeepAlive::Os)
    .worker_max_blocking_threads(16)
    .bind((ip.to_string(), port))?
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

    let metrics = ActixWebMetricsBuilder::new().build().unwrap();

    HttpServer::new(move || {
        let metrics_handler = metrics_handler.clone();
        App::new()
            .wrap(metrics.clone())
            .wrap(Sentry::new())
            .wrap(Logger::default())
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
                    metrics_handler.run_upkeep();
                    async move {
                        let metrics = metrics_handler.render();

                        HttpResponse::Ok()
                            .content_type("text/plain; version=0.0.4")
                            .body(metrics)
                    }
                }),
            )
    })
    .bind((ip.to_string(), port))?
    .run()
    .await?;

    Ok(())
}
