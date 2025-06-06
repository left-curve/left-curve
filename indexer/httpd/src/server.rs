use {
    super::error::Error,
    crate::{context::Context, routes},
    actix_cors::Cors,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
    },
    actix_web_metrics::{ActixWebMetrics, ActixWebMetricsBuilder},
    metrics::{counter, describe_counter},
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    sentry_actix::Sentry,
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

    let metrics = ActixWebMetricsBuilder::new().build().unwrap();

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
            .wrap(metrics.clone())
            .wrap(Sentry::new())
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
    tracing::info!("Starting metrics httpd server at {ip}:{port}");

    let metrics = ActixWebMetricsBuilder::new().build().unwrap();

    // or use:
    // let metrics_handler2 = PrometheusBuilder::new()
    //     .install_recorder()
    //     .expect("failed to install recorder");
    // This allows actix endpoits to use the same metrics as the main application.
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
                web::get().to(|| async {
                    counter!("metrics.root.calls").increment(1);
                    HttpResponse::Ok().body("Metrics server is running")
                }),
            )
            .route(
                "/metrics",
                web::get().to(move || {
                    let metrics_handler = metrics_handler.clone();
                    let metrics_handler2 = metrics_handler2.clone();
                    metrics_handler2.run_upkeep();

                    counter!("metrics.metrics.calls").increment(1);

                    async move {
                        let metrics2 = metrics_handler2.render();
                        let metrics = metrics_handler.render();
                        let combined = format!("{}\n{}", metrics, metrics2);

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
            .service(routes::api::services::api_services())
            .service(routes::graphql::graphql_route())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
