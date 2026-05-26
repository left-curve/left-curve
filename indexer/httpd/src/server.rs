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
        routes::{
            graphql::{GraphqlRequestTimeout, graphql_route},
            index::index,
        },
        subscription_limiter::SubscriptionLimiter,
    },
    grug_types::HttpdConfig,
    sentry_actix::Sentry,
    std::time::Duration,
};
#[cfg(feature = "metrics")]
use {
    crate::middlewares::metrics::init_httpd_metrics,
    actix_web_metrics::ActixWebMetricsBuilder,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    std::{fmt::Display, time::Duration as StdDuration},
};

#[cfg(feature = "metrics")]
const JEMALLOC_REFRESH_INTERVAL: StdDuration = StdDuration::from_secs(10);

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

    let graphql_request_timeout = GraphqlRequestTimeout(Duration::from_secs(
        httpd_config.graphql_request_timeout_secs,
    ));

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

    // Background task that periodically refreshes the jemalloc gauges so they
    // show up in `/metrics`. Detached: runs for the lifetime of the tokio
    // runtime and exits when the process does.
    tokio::spawn(refresh_jemalloc_gauges_forever());

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
            .route("/debug/pprof/heap", web::get().to(dump_heap_pprof))
            .route(
                "/debug/pprof/heap/activate",
                web::post().to(activate_heap_pprof),
            )
            .route(
                "/debug/pprof/heap/deactivate",
                web::post().to(deactivate_heap_pprof),
            )
            .route("/debug/jemalloc/stats", web::get().to(dump_jemalloc_stats))
    })
    .bind((ip.to_string(), port))?
    .run()
    .await?;

    Ok(())
}

// ---- jemalloc heap profiling endpoints ----

/// Returns a pprof-format heap snapshot.
///
/// Requires jemalloc built with profiling (the workspace enables this via the
/// `tikv-jemallocator` `profiling` feature) AND profiling armed at startup via
/// the env var `MALLOC_CONF=prof:true`. Activation can be done at startup
/// (`prof_active:true`) or on demand via `/debug/pprof/heap/activate`.
#[cfg(feature = "metrics")]
async fn dump_heap_pprof() -> HttpResponse {
    let Some(prof_ctl) = jemalloc_pprof::PROF_CTL.as_ref() else {
        return HttpResponse::ServiceUnavailable()
            .body("jemalloc profiling not available; restart with MALLOC_CONF=prof:true");
    };
    let mut prof_ctl = prof_ctl.lock().await;
    if !prof_ctl.activated() {
        return HttpResponse::ServiceUnavailable()
            .body("jemalloc profiling not active; POST /debug/pprof/heap/activate first");
    }
    match prof_ctl.dump_pprof() {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(bytes),
        Err(err) => HttpResponse::InternalServerError().body(format!("dump_pprof failed: {err}")),
    }
}

#[cfg(feature = "metrics")]
async fn activate_heap_pprof() -> HttpResponse {
    let Some(prof_ctl) = jemalloc_pprof::PROF_CTL.as_ref() else {
        return HttpResponse::ServiceUnavailable()
            .body("jemalloc profiling not available; restart with MALLOC_CONF=prof:true");
    };
    let mut prof_ctl = prof_ctl.lock().await;
    match prof_ctl.activate() {
        Ok(()) => HttpResponse::Ok().body("activated"),
        Err(err) => HttpResponse::InternalServerError().body(format!("activate failed: {err}")),
    }
}

#[cfg(feature = "metrics")]
async fn deactivate_heap_pprof() -> HttpResponse {
    let Some(prof_ctl) = jemalloc_pprof::PROF_CTL.as_ref() else {
        return HttpResponse::ServiceUnavailable()
            .body("jemalloc profiling not available; restart with MALLOC_CONF=prof:true");
    };
    let mut prof_ctl = prof_ctl.lock().await;
    match prof_ctl.deactivate() {
        Ok(()) => HttpResponse::Ok().body("deactivated"),
        Err(err) => HttpResponse::InternalServerError().body(format!("deactivate failed: {err}")),
    }
}

// ---- jemalloc arena-level stats dump ----

// jemalloc's `malloc_stats_print`, exposed unprefixed because the workspace
// enables the `unprefixed_malloc_on_supported_platforms` feature on
// tikv-jemallocator. Declared inline to avoid pulling tikv-jemalloc-sys
// explicitly — the symbol is already in the binary via the global allocator.
#[cfg(feature = "metrics")]
unsafe extern "C" {
    unsafe fn malloc_stats_print(
        write_cb: Option<
            unsafe extern "C" fn(opaque: *mut std::ffi::c_void, msg: *const std::ffi::c_char),
        >,
        cbopaque: *mut std::ffi::c_void,
        opts: *const std::ffi::c_char,
    );
}

/// Returns a human-readable, arena-by-arena dump of jemalloc state:
/// dirty/muzzy/purged pages per arena, large allocation histograms, retained
/// vs mapped, decay configuration, etc. Complements the pprof flame graph
/// when diagnosing fragmentation vs allocation growth.
#[cfg(feature = "metrics")]
async fn dump_jemalloc_stats() -> HttpResponse {
    use std::{
        ffi::{CStr, c_char, c_void},
        ptr,
    };

    unsafe extern "C" fn write_cb(opaque: *mut c_void, msg: *const c_char) {
        if msg.is_null() || opaque.is_null() {
            return;
        }
        // SAFETY: jemalloc passes back the same `opaque` pointer we gave it,
        // which we set to a `&mut String`. `msg` is a null-terminated C string
        // produced by jemalloc itself.
        unsafe {
            let s = CStr::from_ptr(msg).to_string_lossy();
            let buf = &mut *(opaque as *mut String);
            buf.push_str(&s);
        }
    }

    // `malloc_stats_print` walks every arena and emits a multi-MB report on
    // a large heap; on a 30 GiB process this can stall the calling thread
    // for hundreds of ms. Run it on the blocking pool so it doesn't tie up
    // an actix worker (which is exactly when we'd want it: during an
    // incident with a bloated heap).
    let result = tokio::task::spawn_blocking(|| {
        let mut buf = String::new();
        // SAFETY: jemalloc invokes the callback synchronously from the
        // calling thread for the duration of this call. `buf` lives on this
        // stack frame until `malloc_stats_print` returns.
        unsafe {
            malloc_stats_print(
                Some(write_cb),
                &mut buf as *mut _ as *mut c_void,
                ptr::null(),
            );
        }
        buf
    })
    .await;

    match result {
        Ok(buf) => HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(buf),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("dump_jemalloc_stats join failed: {err}")),
    }
}

// ---- jemalloc statistics gauges ----

/// Refresh the jemalloc gauges every `JEMALLOC_REFRESH_INTERVAL`.
///
/// Memory-state gauges (refreshed only after a successful `epoch.advance()`):
/// - `allocated`: bytes requested by the application and currently live.
/// - `active`:    bytes in active pages assigned to threads (≈ allocated + small overhead).
/// - `resident`:  bytes mapped into RAM from jemalloc's POV (≈ container RSS).
/// - `mapped`:    bytes mapped in total (including pages not yet returned to the kernel).
/// - `retained`:  bytes of virtual memory retained (mapped but not in
///   `resident`/`active`). A large `retained` indicates virtual-memory
///   fragmentation, where the allocator has unmapped physical pages but kept
///   the virtual reservation.
/// - `metadata`:  bytes consumed by jemalloc's own bookkeeping (arenas, chunks).
///   Should be small (tens of MiB) — sustained growth signals a structural problem.
///
/// Profiling-state gauge (refreshed every tick, independent of stats):
/// - `profiling_active`: 1 if heap sampling is currently on, 0 otherwise.
///   Pair with an alert (`> 0 for 15m`) to catch a forgotten activation —
///   live sampling costs ~5-15% CPU on top of the ~1-3% baseline of armed-but-
///   inactive profiling.
///
/// The ratio `resident / allocated` is the best at-a-glance fragmentation
/// signal: ~1.0 means the process is using what it has; >1.5 means the
/// allocator is holding onto a lot of overhead.
#[cfg(feature = "metrics")]
async fn refresh_jemalloc_gauges_forever() {
    use tikv_jemalloc_ctl::{epoch, stats};

    // Resolve MIBs once; reads via MIB are faster than via the string keys.
    let mibs = match (
        epoch::mib(),
        stats::allocated::mib(),
        stats::active::mib(),
        stats::resident::mib(),
        stats::mapped::mib(),
        stats::retained::mib(),
        stats::metadata::mib(),
    ) {
        (Ok(e), Ok(a), Ok(act), Ok(r), Ok(m), Ok(ret), Ok(meta)) => {
            Some((e, a, act, r, m, ret, meta))
        },
        _ => None,
    };

    let Some((
        epoch_mib,
        allocated_mib,
        active_mib,
        resident_mib,
        mapped_mib,
        retained_mib,
        metadata_mib,
    )) = mibs
    else {
        #[cfg(feature = "tracing")]
        tracing::warn!("Failed to resolve jemalloc stat MIBs; gauges disabled");
        return;
    };

    let mut interval = tokio::time::interval(JEMALLOC_REFRESH_INTERVAL);

    loop {
        interval.tick().await;

        // Profiling-active state lives in `jemalloc_pprof::PROF_CTL`, not in
        // the stats epoch — update it unconditionally so a forgotten
        // activation is observable even if stats reads fail.
        let profiling_active = match jemalloc_pprof::PROF_CTL.as_ref() {
            Some(prof_ctl) if prof_ctl.lock().await.activated() => 1.0,
            _ => 0.0,
        };
        metrics::gauge!("jemalloc_profiling_active").set(profiling_active);

        // Stats are cached until the epoch is advanced.
        if let Err(_err) = epoch_mib.advance() {
            #[cfg(feature = "tracing")]
            tracing::debug!(%_err, "jemalloc epoch advance failed");
            continue;
        }

        if let Ok(v) = allocated_mib.read() {
            metrics::gauge!("jemalloc_allocated_bytes").set(v as f64);
        }
        if let Ok(v) = active_mib.read() {
            metrics::gauge!("jemalloc_active_bytes").set(v as f64);
        }
        if let Ok(v) = resident_mib.read() {
            metrics::gauge!("jemalloc_resident_bytes").set(v as f64);
        }
        if let Ok(v) = mapped_mib.read() {
            metrics::gauge!("jemalloc_mapped_bytes").set(v as f64);
        }
        if let Ok(v) = retained_mib.read() {
            metrics::gauge!("jemalloc_retained_bytes").set(v as f64);
        }
        if let Ok(v) = metadata_mib.read() {
            metrics::gauge!("jemalloc_metadata_bytes").set(v as f64);
        }
    }
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
