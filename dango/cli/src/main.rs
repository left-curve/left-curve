use {
    sentry::integrations::tracing::layer as sentry_layer,
    tracing_subscriber::{fmt, prelude::*, registry},
};

mod config;
mod db;
mod home_directory;
mod keys;
mod prompt;
mod query;
mod start;
#[cfg(feature = "testing")]
mod test;
mod tx;

#[cfg(feature = "testing")]
use crate::test::TestCmd;
use {
    crate::{
        db::DbCmd, home_directory::HomeDirectory, keys::KeysCmd, query::QueryCmd, start::StartCmd,
        tx::TxCmd,
    },
    clap::Parser,
    config::Config,
    config_parser::parse_config,
    std::path::PathBuf,
    tracing::metadata::LevelFilter,
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

    if cfg.sentry.enabled {
        let _sentry_guard = sentry::init((cfg.sentry.dsn, sentry::ClientOptions {
            environment: Some(cfg.sentry.environment.into()),
            release: sentry::release_name!(),
            sample_rate: cfg.sentry.sample_rate,
            traces_sample_rate: cfg.sentry.traces_sample_rate,
            debug: true,
            ..Default::default()
        }));

        sentry::configure_scope(|scope| {
            scope.set_tag("chain-id", &cfg.transactions.chain_id);
        });

        let max_level = cfg.log_level.parse::<LevelFilter>()?;

        registry()
            .with(fmt::layer().with_filter(max_level))
            .with(sentry_layer())
            .init();

        tracing::info!("Sentry initialized");
    } else {
        // Set up tracing.
        tracing_subscriber::fmt()
            .with_max_level(cfg.log_level.parse::<LevelFilter>()?)
            .init();
    }

    match cli.command {
        Command::Db(cmd) => cmd.run(app_dir),
        Command::Keys(cmd) => cmd.run(app_dir.keys_dir()),
        Command::Query(cmd) => cmd.run(app_dir).await,
        Command::Start(cmd) => cmd.run(app_dir).await,
        #[cfg(feature = "testing")]
        Command::Test(cmd) => cmd.run().await,
        Command::Tx(cmd) => cmd.run(app_dir).await,
    }
}
