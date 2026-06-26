use {
    crate::config::HttpdConfig,
    actix_cors::Cors,
    actix_web::{App, HttpResponse, HttpServer, Responder, rt::System, web},
    anyhow::{Context as _, bail},
    async_graphql::{
        ObjectType, Schema, SubscriptionType,
        http::{GraphQLPlaygroundConfig, playground_source},
    },
    async_graphql_actix_web::{GraphQLRequest, GraphQLResponse},
    dango_indexer_historical_types::AnyResult,
    futures::future::BoxFuture,
    tokio::sync::oneshot,
};

/// Serve a pre-built schema until the process stops, as a boxed task.
///
/// Returned type-erased (`BoxFuture`) so the app can supervise it without
/// knowing the schema's concrete type. actix-web's server future is `!Send`,
/// so it runs on a dedicated thread with its own actix `System`; the outcome
/// comes back over a oneshot, and only that `Send` receiver is held across the
/// await — so the returned future is `Send` and spawns like any other task.
pub fn serve<Q, M, S>(
    schema: Schema<Q, M, S>,
    config: HttpdConfig,
) -> BoxFuture<'static, AnyResult<()>>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    Box::pin(async move {
        let (tx, rx) = oneshot::channel();

        std::thread::Builder::new()
            .name("historical-httpd".to_string())
            .spawn(move || {
                let outcome = System::new().block_on(serve_actix(schema, config));
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
async fn serve_actix<Q, M, S>(schema: Schema<Q, M, S>, config: HttpdConfig) -> AnyResult<()>
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    let bind = config.bind.clone();

    #[cfg(feature = "tracing")]
    tracing::info!(%bind, "historical indexer httpd listening");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(schema.clone()))
            // Dev-permissive CORS for now; lock down via config later.
            .wrap(Cors::permissive())
            .route("/graphql", web::post().to(graphql::<Q, M, S>))
            .route("/graphql", web::get().to(playground))
            .route("/up", web::get().to(up))
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

/// Execute a GraphQL request (query / mutation / subscription init).
async fn graphql<Q, M, S>(
    schema: web::Data<Schema<Q, M, S>>,
    req: GraphQLRequest,
) -> GraphQLResponse
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    // End-to-end request stats: in-flight gauge bracketing the execution, plus a
    // count and a latency histogram. async-graphql folds resolver errors into the
    // response (no panic), so the decrement always runs.
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();
    #[cfg(feature = "metrics")]
    metrics::gauge!(crate::metrics::GRAPHQL_IN_FLIGHT).increment(1.0);

    let response = schema.execute(req.into_inner()).await;

    #[cfg(feature = "metrics")]
    {
        metrics::gauge!(crate::metrics::GRAPHQL_IN_FLIGHT).decrement(1.0);
        metrics::counter!(crate::metrics::GRAPHQL_REQUESTS).increment(1);
        metrics::histogram!(crate::metrics::GRAPHQL_REQUEST_DURATION)
            .record(start.elapsed().as_secs_f64());
    }

    response.into()
}

/// Serve the in-browser GraphQL playground, pointed at `/graphql`.
async fn playground() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Liveness probe.
async fn up() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}
