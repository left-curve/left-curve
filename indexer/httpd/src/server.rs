use {
    super::error::Error,
    crate::{context::Context, routes},
    actix_cors::Cors,
    actix_web::{
        App, HttpResponse, HttpServer, http,
        middleware::{Compress, Logger},
        web::{self, ServiceConfig},
    },
    grug_httpd::{
        routes::{graphql::graphql_route, index::index},
        subscription_limiter::SubscriptionLimiter,
    },
    grug_types::HttpdConfig,
    sentry_actix::Sentry,
};
#[cfg(feature = "metrics")]
use {crate::middlewares::metrics::init_httpd_metrics, actix_web_metrics::ActixWebMetricsBuilder};

/// Run the HTTP server, includes GraphQL and REST endpoints.
pub async fn run_server<CA, GS>(
    httpd_config: &HttpdConfig,
    context: Context,
    config_app: CA,
    build_schema: fn(Context) -> GS,
) -> Result<(), Error>
where
    CA: Fn(Context, GS) -> Box<dyn Fn(&mut ServiceConfig)> + Clone + Send + 'static,
    GS: Clone + Send + 'static,
{
    let graphql_schema = build_schema(context.clone());

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

    let cors_allowed_origin = httpd_config.cors_allowed_origin.clone();
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

        app.app_data(web::Data::new(subscription_limiter.clone()))
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
    .bind((&*httpd_config.ip, httpd_config.port))?
    .run()
    .await?;

    Ok(())
}

pub fn config_app<G>(app_ctx: Context, graphql_schema: G) -> Box<dyn Fn(&mut ServiceConfig)>
where
    G: Clone + 'static,
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.service(index)
            .service(routes::index::up)
            .service(grug_httpd::routes::index::requester_ip)
            .service(routes::blocks::services())
            .service(graphql_route::<
                crate::graphql::query::IndexerQuery,
                crate::graphql::mutation::IndexerMutation,
                crate::graphql::subscription::IndexerSubscription,
            >())
            .default_service(web::to(HttpResponse::NotFound))
            .app_data(web::Data::new(app_ctx.clone()))
            .app_data(web::Data::new(graphql_schema.clone()));
    })
}
