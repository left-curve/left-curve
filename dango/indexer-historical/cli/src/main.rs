//! `indexer-historical` — the historical indexer's command-line entry point.
//!
//! A small `clap` front end (structure inspired by `dango-cli`, kept minimal):
//! a global `--home` and one subcommand per mode of operation. For now the only
//! command is [`start`](start), which boots the ingest + read-API service.
//!
//! Logs are a single `fmt` layer honoring `RUST_LOG` (the OTLP / Sentry tracing
//! export `dango-cli` wires can be layered on here later); metrics are a global
//! Prometheus recorder installed in `start` and scraped at `/metrics` — see
//! `design/observability.md`.

mod activity;
mod config;
mod db;
mod home_directory;
mod metrics;
mod read_api;
mod source;
mod start;

use {
    crate::{home_directory::HomeDirectory, start::StartCmd},
    clap::{CommandFactory, FromArgMatches, Parser, Subcommand},
    std::{path::PathBuf, sync::LazyLock},
    tracing_subscriber::{EnvFilter, prelude::*},
};

/// Crate version with the embedded git commit, surfaced by `--version`.
static VERSION_WITH_COMMIT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{} ({})",
        env!("CARGO_PKG_VERSION"),
        dango_primitives::GIT_COMMIT
    )
});

/// The historical indexer: ingest blocks from a `BlockSource` and serve the
/// projections' GraphQL read API.
#[derive(Parser)]
#[command(author, about, next_display_order = None)]
struct Cli {
    /// Directory for config and local state [default: ~/.indexer-historical]
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the indexer: ingest blocks and serve the read API.
    Start(StartCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse args, overriding `--version` to include the git commit.
    let cli = {
        let cmd = Cli::command().version(VERSION_WITH_COMMIT.as_str());
        let matches = cmd.get_matches();
        Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
    };

    // Minimal tracing: a fmt layer honoring `RUST_LOG` (default `info`). The
    // OTLP / Sentry export layers can be composed in here later.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let home = HomeDirectory::new_or_default(cli.home)?;

    match cli.command {
        Command::Start(cmd) => cmd.run(home).await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `clap` derive sanity: the command tree is internally consistent (no
    /// conflicting args, valid subcommands). A cheap guard as commands grow.
    #[test]
    fn cli_is_well_formed() {
        Cli::command().debug_assert();
    }
}
