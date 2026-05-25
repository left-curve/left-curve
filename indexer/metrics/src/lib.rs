use {
    actix_web::{
        App, HttpResponse, HttpServer,
        middleware::{Compress, Logger},
        web,
    },
    actix_web_metrics::ActixWebMetricsBuilder,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    sentry_actix::Sentry,
    std::{fmt::Display, io, time::Duration},
};

/// How often the background task refreshes the jemalloc gauges.
const JEMALLOC_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

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

    // Background task that periodically refreshes the jemalloc gauges so they
    // show up in `/metrics`. Detached: it runs for the lifetime of the
    // tokio runtime and exits when the process does.
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
    })
    .bind((ip.to_string(), port))?
    .run()
    .await?;

    Ok(())
}

// ---- jemalloc heap profiling endpoints ----

/// Returns a pprof-format heap snapshot.
///
/// Requires jemalloc to be built with profiling (the workspace enables this
/// via the `tikv-jemallocator` `profiling` feature) AND profiling armed at
/// startup via the env var `MALLOC_CONF=prof:true`. Activation can be done at
/// startup (`prof_active:true`) or on demand via `/debug/pprof/heap/activate`.
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

// ---- jemalloc statistics gauges ----

/// Refresh the jemalloc gauges every `JEMALLOC_REFRESH_INTERVAL`.
///
/// These four numbers are the cheapest way to understand the allocator's
/// state in real time:
/// - `allocated`: bytes requested by the application and currently live.
/// - `active`:    bytes in active pages assigned to threads (≈ allocated + small overhead).
/// - `resident`:  bytes mapped into RAM from jemalloc's POV (≈ container RSS).
/// - `mapped`:    bytes mapped in total (including pages not yet returned to the kernel).
///
/// The ratio `resident / allocated` is the best at-a-glance fragmentation
/// signal: ~1.0 means the process is using what it has; >1.5 means the
/// allocator is holding onto a lot of overhead.
async fn refresh_jemalloc_gauges_forever() {
    use tikv_jemalloc_ctl::{epoch, stats};

    // Resolve MIBs once; reads via MIB are faster than via the string keys.
    let mibs = match (
        epoch::mib(),
        stats::allocated::mib(),
        stats::active::mib(),
        stats::resident::mib(),
        stats::mapped::mib(),
    ) {
        (Ok(e), Ok(a), Ok(act), Ok(r), Ok(m)) => Some((e, a, act, r, m)),
        _ => None,
    };

    let Some((epoch_mib, allocated_mib, active_mib, resident_mib, mapped_mib)) = mibs else {
        #[cfg(feature = "tracing")]
        tracing::warn!("Failed to resolve jemalloc stat MIBs; gauges disabled");
        return;
    };

    let mut interval = tokio::time::interval(JEMALLOC_REFRESH_INTERVAL);

    loop {
        interval.tick().await;

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
    }
}
