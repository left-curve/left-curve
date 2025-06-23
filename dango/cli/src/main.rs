mod config;
mod db;
mod home_directory;
mod indexer;
mod keys;
mod prompt;
mod query;
mod start;
#[cfg(feature = "testing")]
mod test;
mod tracing_filter;
mod tx;

#[cfg(feature = "testing")]
use crate::test::TestCmd;
use {
    crate::{
        db::DbCmd, home_directory::HomeDirectory, indexer::IndexerCmd, keys::KeysCmd,
        query::QueryCmd, start::StartCmd, tracing_filter::SuppressingLevelFilter, tx::TxCmd,
    },
    clap::Parser,
    config::Config,
    config_parser::parse_config,
    sentry::integrations::tracing::layer as sentry_layer,
    std::path::PathBuf,
    tracing_subscriber::{fmt::format::FmtSpan, prelude::*},
};

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

    // Set up tracing, depending on whether Sentry is enabled or not.
    let filter = SuppressingLevelFilter::from_inner(cfg.log_level.parse()?);

    // Create the base environment filter
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| cfg.log_level.clone().into()); // Default to cfg.log_level if RUST_LOG not set

    // Create the fmt layer based on the configured format
    let fmt_layer = match cfg.log_format {
        config::LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .boxed(),
        config::LogFormat::Text => tracing_subscriber::fmt::layer().boxed(),
    };

    if cfg.sentry.enabled {
        let _sentry_guard = sentry::init((cfg.sentry.dsn, sentry::ClientOptions {
            environment: Some(cfg.sentry.environment.into()),
            release: sentry::release_name!(),
            sample_rate: cfg.sentry.sample_rate,
            traces_sample_rate: cfg.sentry.traces_sample_rate,
            ..Default::default()
        }));

        sentry::configure_scope(|scope| {
            scope.set_tag("chain-id", &cfg.transactions.chain_id);
        });

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.with_filter(filter))
            .with(sentry_layer())
            .init();

        tracing::info!("Sentry initialized");
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    match cli.command {
        Command::Db(cmd) => cmd.run(app_dir),
        Command::Indexer(cmd) => cmd.run(app_dir).await,
        Command::Keys(cmd) => cmd.run(app_dir.keys_dir()),
        Command::Query(cmd) => cmd.run(app_dir).await,
        Command::Start(cmd) => cmd.run(app_dir).await,
        #[cfg(feature = "testing")]
        Command::Test(cmd) => cmd.run().await,
        Command::Tx(cmd) => cmd.run(app_dir).await,
    }
}
