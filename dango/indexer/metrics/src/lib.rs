use {
    actix_web::{
        App, HttpResponse, HttpServer,
        middleware::{Compress, Logger},
        web,
    },
    actix_web_metrics::ActixWebMetricsBuilder,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    sentry_actix::Sentry,
    std::{fmt::Display, io},
};

/// Run the metrics HTTP server
pub async fn run_metrics_server<I>(
    ip: I,
    port: u16,
    metrics_handler: PrometheusHandle,
) -> Result<(), io::Error>
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
