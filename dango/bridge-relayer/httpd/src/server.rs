use {
    super::error::Error,
    crate::{context::Context, migrations, routes},
    actix_cors::Cors,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self},
    },
    dango_types::{
        bitcoin::{Config as BridgeConfig, QueryConfigRequest},
        config::AppConfig,
    },
    grug::{ClientWrapper, QueryClientExt},
    indexer_client::HttpClient,
    metrics_exporter_prometheus::PrometheusBuilder,
    sea_orm::DatabaseConnection,
    sea_orm_migration::MigratorTrait,
    sentry_actix::Sentry,
    std::{fmt::Display, sync::Arc},
};
#[cfg(feature = "metrics")]
use {actix_web_metrics::ActixWebMetricsBuilder, metrics_exporter_prometheus::PrometheusHandle};

/// Run the bridge relayer HTTP server
pub async fn run_server<I>(
    ip: I,
    port: u16,
    cors_allowed_origin: Option<String>,
    context: Context,
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
            .wrap(cors)
            .app_data(web::Data::new(context.clone()));

        #[cfg(feature = "metrics")]
        let app = app.wrap(metrics.clone());

        app.route(
            "/health",
            web::get().to(|| async { HttpResponse::Ok().body("Bridge relayer server is healthy") }),
        )
        .service(routes::deposit_address)
        .service(routes::deposit_addresses)
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

pub async fn get_bridge_config(dango_url: String) -> Result<BridgeConfig, Error> {
    // Initialize Dango client.
    let dango_client = ClientWrapper::new(Arc::new(HttpClient::new(dango_url)?));

    // Query the bitcoin bridge contract address.
    let bitcoin_bridge = dango_client
        .query_app_config::<AppConfig>(None)
        .await?
        .addresses
        .bitcoin;

    // Load bitcoin bridge config from contract.
    let bridge_config = dango_client
        .query_wasm_smart(bitcoin_bridge, QueryConfigRequest {}, None)
        .await?;

    Ok(bridge_config)
}

pub async fn run_servers(bridge_config: BridgeConfig, db: DatabaseConnection) -> Result<(), Error> {
    // Initialize metrics handler.
    // This should be done as soon as possible to capture all events.
    let metrics_handler = PrometheusBuilder::new().install_recorder()?;

    // Run migrations.
    #[cfg(feature = "tracing")]
    tracing::info!("running migrations");
    migrations::Migrator::up(&db, None).await?;
    #[cfg(feature = "tracing")]
    tracing::info!("ran migrations successfully");

    // Create context.
    let context = Context::new(bridge_config, db);

    // Run the server
    let server = run_server("127.0.0.1", 8080, None, context);
    let metrics = run_metrics_server("127.0.0.1", 8081, metrics_handler);

    tokio::try_join!(server, metrics)?;

    Ok(())
}
