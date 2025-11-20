mod config;
mod db;
mod home_directory;
mod indexer;
mod keys;
mod prompt;
mod query;
mod start;
mod telemetry;
mod tendermint;
#[cfg(feature = "testing")]
mod test;
mod tx;

#[cfg(feature = "testing")]
use crate::test::TestCmd;
use {
    crate::{
        db::DbCmd, home_directory::HomeDirectory, indexer::IndexerCmd, keys::KeysCmd,
        query::QueryCmd, start::StartCmd, tendermint::TendermintCmd, tx::TxCmd,
    },
    clap::Parser,
    config::Config,
    config_parser::parse_config,
    opentelemetry::{KeyValue, trace::TracerProvider},
    opentelemetry_otlp::{ExportConfig, Protocol, SpanExporter, WithExportConfig},
    opentelemetry_sdk::{Resource, trace as sdktrace},
    sentry::integrations::tracing::layer as sentry_layer,
    std::path::PathBuf,
    tracing_opentelemetry::{layer as otel_layer, OpenTelemetrySpanExt},
    tracing_subscriber::{
        fmt::format::FmtSpan,
        layer::{Context as LayerContext},
        prelude::*,
        registry::LookupSpan,
        Layer as _,
    },
};

// Enrich the current span with OpenTelemetry trace/span ids so fmt logging can include them.
struct TraceIdLayer;

impl<S> tracing_subscriber::Layer<S> for TraceIdLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_enter(&self, id: &tracing_core::span::Id, ctx: LayerContext<'_, S>) {
        // When a span is entered, record otel trace/span ids onto it (if available).
        if let Some(_span_ref) = ctx.span(id) {
            let otel_ctx = tracing::Span::current().context();
            let sc = otel_ctx.span().span_context();
            if sc.is_valid() {
                let span = tracing::Span::current();
                span.record("trace_id", &tracing::field::display(sc.trace_id()));
                span.record("span_id", &tracing::field::display(sc.span_id()));
            }
        }
    }

    fn on_event(&self, _event: &tracing_core::Event<'_>, _ctx: LayerContext<'_, S>) {
        // Best-effort: also record onto whatever span is current when an event happens.
        let otel_ctx = tracing::Span::current().context();
        let sc = otel_ctx.span().span_context();
        if sc.is_valid() {
            let span = tracing::Span::current();
            span.record("trace_id", &tracing::field::display(sc.trace_id()));
            span.record("span_id", &tracing::field::display(sc.span_id()));
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    /// Directory for the physical database [default: ~/.dango]
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Manage the database
    #[command(subcommand, next_display_order = None)]
    Db(DbCmd),

    /// Indexer related commands
    Indexer(IndexerCmd),

    /// Manage keys
    #[command(subcommand, next_display_order = None)]
    Keys(KeysCmd),

    /// Make a query [alias: q]
    #[command(next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Start the node
    Start(StartCmd),

    /// Interact with Tendermint RPC [alias: tm]
    #[command(next_display_order = None, alias = "tm")]
    Tendermint(TendermintCmd),

    /// Run test
    #[cfg(feature = "testing")]
    Test(TestCmd),

    /// Send transactions
    #[command(next_display_order = None)]
    Tx(TxCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments.
    let cli = Cli::parse();

    // Find the home directory from the CLI `--home` flag.
    let app_dir = HomeDirectory::new_or_default(cli.home)?;

    // Parse the config file.
    let cfg: Config = parse_config(app_dir.config_file())?;

    // Create the base environment filter
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| cfg.log_level.clone().into()); // Default to `cfg.log_level` if `RUST_LOG` not set.

    // Create the fmt layer based on the configured format
    let fmt_layer = match cfg.log_format {
        config::LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .with_span_events(FmtSpan::NONE)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            // include the current span so recorded trace_id/span_id appear in JSON output
            .with_current_span(true)
            .boxed(),
        config::LogFormat::Text => tracing_subscriber::fmt::layer()
            // include current span in text logs as well
            .with_current_span(true)
            .boxed(),
    };

    // Optionally build an OpenTelemetry layer if tracing export is enabled.
    let otel_layer_opt = if cfg.trace.enabled {
        let mut attrs = vec![
            // Keep existing chain id for querying
            KeyValue::new("chain.id", cfg.transactions.chain_id.clone()),
            // Required: service.instance.id — default to chain_id
            KeyValue::new("service.instance.id", cfg.transactions.chain_id.clone()),
        ];

        // Required: service.namespace — priority: SERVICE_NAMESPACE > DEPLOY_ENV > sentry.environment > DEPLOYMENT_NAME
        let service_namespace = std::env::var("SERVICE_NAMESPACE")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| std::env::var("DEPLOY_ENV").ok().filter(|s| !s.is_empty()))
            .or_else(|| {
                (!cfg.sentry.environment.is_empty()).then(|| cfg.sentry.environment.clone())
            })
            .or_else(|| {
                std::env::var("DEPLOYMENT_NAME")
                    .ok()
                    .filter(|s| !s.is_empty())
            });
        if let Some(ns) = service_namespace {
            attrs.push(KeyValue::new("service.namespace", ns));
        }

        // Optional: deployment.environment — prefer DEPLOY_ENV, else sentry.environment
        if let Some(env) = std::env::var("DEPLOY_ENV")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                (!cfg.sentry.environment.is_empty()).then(|| cfg.sentry.environment.clone())
            })
        {
            attrs.push(KeyValue::new("deployment.environment", env));
        }

        // Optional: host.name — read from HOSTNAME if provided
        if let Ok(host) = std::env::var("HOSTNAME") {
            if !host.is_empty() {
                attrs.push(KeyValue::new("host.name", host));
            }
        }

        let resource = Resource::builder()
            .with_service_name("dango")
            .with_attributes(attrs)
            .build();

        // Build exporter and tracer provider
        // Build exporter via selected OTLP protocol (gRPC or HTTP).
        let exporter = match cfg.trace.protocol {
            config::TraceProtocol::OtlpGrpc => {
                let export_config = ExportConfig {
                    endpoint: Some(cfg.trace.endpoint.clone()),
                    protocol: Protocol::Grpc,
                    ..Default::default()
                };
                SpanExporter::builder()
                    .with_tonic()
                    .with_export_config(export_config)
                    .build()?
            },
            config::TraceProtocol::OtlpHttp => {
                let export_config = ExportConfig {
                    endpoint: Some(cfg.trace.endpoint.clone()),
                    protocol: Protocol::HttpBinary,
                    ..Default::default()
                };
                SpanExporter::builder()
                    .with_http()
                    .with_export_config(export_config)
                    .build()?
            },
        };

        let provider = sdktrace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build();

        // Register provider in a global OnceLock so signal handlers can shut it down.
        let tracer = provider.tracer("dango");
        crate::telemetry::set_provider(provider);
        Some(otel_layer().with_tracer(tracer))
    } else {
        None
    };

    let mut _sentry_guard: Option<sentry::ClientInitGuard> = None;
    let sentry_layer = if cfg.sentry.enabled {
        let guard = sentry::init((cfg.sentry.dsn, sentry::ClientOptions {
            environment: Some(cfg.sentry.environment.clone().into()),
            release: sentry::release_name!(),
            sample_rate: cfg.sentry.sample_rate,
            traces_sample_rate: cfg.sentry.traces_sample_rate,
            ..Default::default()
        }));
        _sentry_guard = Some(guard);

        sentry::configure_scope(|scope| {
            scope.set_tag("chain-id", &cfg.transactions.chain_id);
        });
        Some(sentry_layer())
    } else {
        None
    };

    // Compose the subscriber with optional layers (Option implements Layer)
    tracing_subscriber::registry()
        .with(env_filter)
        // Enrich spans/events with otel trace/span ids for log correlation
        .with(TraceIdLayer)
        .with(fmt_layer)
        .with(sentry_layer)
        .with(otel_layer_opt)
        .init();

    // Emit startup logs now that the subscriber is initialized.
    if cfg.sentry.enabled {
        tracing::info!("Sentry initialized");
    } else {
        tracing::info!("Sentry is disabled");
    }
    if cfg.trace.enabled {
        tracing::info!(endpoint = %cfg.trace.endpoint, protocol = ?cfg.trace.protocol, "OpenTelemetry OTLP exporter initialized");
    } else {
        tracing::info!("OpenTelemetry OTLP exporter is disabled");
    }

    match cli.command {
        Command::Db(cmd) => cmd.run(app_dir)?,
        Command::Indexer(cmd) => cmd.run(app_dir).await?,
        Command::Keys(cmd) => cmd.run(app_dir.keys_dir())?,
        Command::Query(cmd) => cmd.run(app_dir).await?,
        Command::Start(cmd) => cmd.run(app_dir).await?,
        Command::Tendermint(cmd) => cmd.run(app_dir).await?,
        #[cfg(feature = "testing")]
        Command::Test(cmd) => cmd.run(app_dir).await?,
        Command::Tx(cmd) => cmd.run(app_dir).await?,
    }

    // Flush and shutdown the tracer provider (if set) to avoid losing spans.
    crate::telemetry::shutdown();
    // Flush Sentry transport too.
    crate::telemetry::shutdown_sentry();

    Ok(())
}
