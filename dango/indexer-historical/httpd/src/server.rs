use {
    crate::{config::HttpdConfig, error::ApiError},
    actix_cors::Cors,
    actix_web::{App, HttpResponse, HttpServer, Responder, dev::Service as _, rt::System, web},
    anyhow::{Context as _, bail},
    dango_indexer_historical_block_source::BlockSource,
    dango_indexer_historical_types::AnyResult,
    futures::future::BoxFuture,
    sea_orm::DatabaseConnection,
    std::sync::Arc,
    tokio::sync::oneshot,
};

/// Mounts the projections' read routes onto an actix
/// [`ServiceConfig`](web::ServiceConfig).
///
/// The app builds one of these — a closure that, per worker, asks every
/// projection for its `services()` scopes and registers them — and hands it to
/// [`serve`], which calls it when it builds each worker's app (hence `Fn`). The
/// handlers the scopes mount reach the shared Postgres pool and block source
/// through actix app data, injected here, so this stays projection-agnostic: the
/// httpd never names a projection, it just applies whatever configurator the app
/// gives it. Mirrors the in-process indexer's `config_app`.
pub type Configurator = Arc<dyn Fn(&mut web::ServiceConfig) + Send + Sync>;

/// Serve the read API until the process stops, as a boxed task.
///
/// Builds an actix-web server from the shared read handles and the app's route
/// configurator: the Postgres pool and the block source are injected as app data
/// (handlers pull them with `web::Data`), the core `GET /block/{height}` and
/// `GET /up` routes are mounted directly, and `configure` adds the projections'
/// feed scopes.
///
/// Returned type-erased (`BoxFuture`) so the app supervises it like any other
/// task. actix-web's server future is `!Send`, so it runs on a dedicated thread
/// with its own actix `System`; the outcome comes back over a oneshot, and only
/// that `Send` receiver is held across the await — so the returned future is
/// `Send`.
pub fn serve(
    config: HttpdConfig,
    db: DatabaseConnection,
    source: Arc<dyn BlockSource>,
    configure: Configurator,
) -> BoxFuture<'static, AnyResult<()>> {
    Box::pin(async move {
        let (tx, rx) = oneshot::channel();

        std::thread::Builder::new()
            .name("historical-httpd".to_string())
            .spawn(move || {
                let outcome = System::new().block_on(serve_actix(config, db, source, configure));
                // A gone receiver just means the app is already tearing down.
                let _ = tx.send(outcome);
            })
            .context("spawning the httpd thread")?;

        match rx.await {
            Ok(outcome) => outcome,
            Err(_) => bail!("httpd thread terminated without reporting an outcome"),
        }
    })
}

/// Mount the routes and drive the actix server. Runs on the dedicated httpd
/// thread, inside its actix `System`.
async fn serve_actix(
    config: HttpdConfig,
    db: DatabaseConnection,
    source: Arc<dyn BlockSource>,
    configure: Configurator,
) -> AnyResult<()> {
    let bind = config.bind.clone();

    #[cfg(feature = "tracing")]
    tracing::info!(%bind, "historical indexer httpd listening");

    HttpServer::new(move || {
        // The factory runs once per worker; clone the shared handles and the
        // route configurator into each worker's app.
        let configure = configure.clone();
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(source.clone()))
            // A malformed argument is a 400 with our JSON envelope, uniformly:
            // actix's default for a bad path param is a 404 (and for a bad query
            // a plain-text 400), so map both onto `ApiError::bad_request`.
            .app_data(web::PathConfig::default().error_handler(|err, _req| {
                ApiError::bad_request(format!("invalid path parameter: {err}")).into()
            }))
            .app_data(web::QueryConfig::default().error_handler(|err, _req| {
                ApiError::bad_request(format!("invalid query parameter: {err}")).into()
            }))
            // Dev-permissive CORS for now; lock down via config later.
            .wrap(Cors::permissive())
            // End-to-end request stats: an in-flight gauge bracketing the call,
            // plus a count and a latency histogram. A no-op without `metrics`.
            .wrap_fn(|req, srv| {
                #[cfg(feature = "metrics")]
                let start = std::time::Instant::now();
                #[cfg(feature = "metrics")]
                metrics::gauge!(crate::metrics::HTTP_IN_FLIGHT).increment(1.0);
                let fut = srv.call(req);
                async move {
                    let res = fut.await;
                    #[cfg(feature = "metrics")]
                    {
                        metrics::gauge!(crate::metrics::HTTP_IN_FLIGHT).decrement(1.0);
                        metrics::counter!(crate::metrics::HTTP_REQUESTS).increment(1);
                        metrics::histogram!(crate::metrics::HTTP_REQUEST_DURATION)
                            .record(start.elapsed().as_secs_f64());
                    }
                    res
                }
            })
            // Core routes: the block-by-height read (not a projection) and the
            // liveness probe.
            .route("/block/{height}", web::get().to(block))
            .route("/up", web::get().to(up))
            // The projections' feed scopes, mounted by the app's configurator.
            .configure(move |cfg| (*configure)(cfg))
    })
    // The app owns process signals; the server simply stops with the process.
    .disable_signals()
    .bind(&bind)
    .with_context(|| format!("binding httpd to {bind}"))?
    .run()
    .await
    .context("httpd server stopped with an error")?;

    Ok(())
}

// ---- handlers ----

/// The full block at `height` — metadata, transactions, and execution outcome
/// (`{ block, outcome }`) — read straight from the configured
/// [`BlockSource`]. `404` when the source does not hold that height (below its
/// backfill floor, or not yet ingested); the body is the canonical block JSON,
/// the same shape the node's `/block/full/{height}` route returns.
async fn block(
    source: web::Data<Arc<dyn BlockSource>>,
    height: web::Path<u64>,
) -> actix_web::Result<HttpResponse> {
    let block = source
        .get(height.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    match block {
        Some(block) => Ok(HttpResponse::Ok().json(block)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

/// Liveness probe.
async fn up() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}
