//! The historical indexer's configuration.
//!
//! Parsed from `<home>/config/app.toml` by
//! [`dango_config_parser::parse_config`], which layers **environment
//! overrides** on top: any field is settable via `SECTION__FIELD`
//! (`__`-separated, e.g. `POSTGRES__URL`, `BLOCK_SOURCE__LIVE_URL`), mirroring
//! `dango-cli`. So a deployment can keep secrets (the Postgres URL) out of the
//! committed TOML and inject them from the environment.
//!
//! Only the **deployment-specific** choices live here. What projections run and
//! the block-source tuning knobs (buffer sizes, intervals, timeouts) are
//! compiled in — see `start`.

use {serde::Deserialize, std::path::PathBuf};

/// Top-level config. `postgres` and `block_source` are required; `httpd`
/// defaults to an enabled server on `0.0.0.0:8080`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub postgres: PostgresConfig,
    pub block_source: BlockSourceConfig,
    #[serde(default)]
    pub httpd: HttpdConfig,
}

/// Postgres connection for the committer (the projection cursors + the
/// projections' own tables); the same pool backs the read API's table queries.
/// Named explicitly `postgres` (not `database`) so a future `[clickhouse]`
/// section can sit beside it unambiguously once ClickHouse-backed projections
/// land.
#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    /// libpq URL, e.g. `postgres://user@host/db`. Secret — prefer injecting it
    /// via `POSTGRES__URL` rather than committing it.
    pub url: String,
    /// Connection-pool size. Default 10.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

/// Which block source to run, selected by `type`. The rest of the app is
/// agnostic to the choice — it only ever sees an `Arc<dyn BlockSource>`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockSourceConfig {
    /// Detached host: bounded backfill from a sentinel node, live tail from a
    /// node's `full_block` stream, persisted in a local RocksDB store.
    Remote(RemoteSourceConfig),
    /// Co-located with a dango node: reads its indexer cache and httpd.
    Local(LocalSourceConfig),
}

impl BlockSourceConfig {
    /// A short label for logs — never the URLs / paths.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Remote(_) => "remote",
            Self::Local(_) => "local",
        }
    }

    /// The backfill fetcher kind for a remote source (for logs); `None` for a
    /// local source, which has no fetcher.
    pub fn fetcher_kind(&self) -> Option<&'static str> {
        match self {
            Self::Remote(remote) => Some(remote.fetcher.kind()),
            Self::Local(_) => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteSourceConfig {
    /// RocksDB directory for the persisted block store. A relative path resolves
    /// against `--home`.
    pub store_path: PathBuf,
    /// Base URL of the node httpd for the live `full_block` tail. The `sentinel`
    /// fetcher reuses this same URL for backfill (same node), so it carries no
    /// URL of its own; a different fetcher kind (e.g. `s3`) brings its own.
    pub live_url: String,
    /// Which backfill fetcher to use. Defaults to `sentinel`.
    #[serde(default)]
    pub fetcher: FetcherConfig,
}

/// Backfill fetcher for a remote source, selected by `type`. Only `sentinel`
/// exists today; an S3 archive fetcher is planned, hence the tagged enum.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FetcherConfig {
    /// Bounded backfill from a sentinel node's `/block/full/range`. Reuses the
    /// source's `live_url` (same node as the live tail), so it has no URL field.
    #[default]
    Sentinel,
    // Future: `S3(S3FetcherConfig)` — backfill from an archive bucket with its
    // own bucket / region / prefix; the live tail still uses `live_url`.
}

impl FetcherConfig {
    /// A short label for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Sentinel => "sentinel",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalSourceConfig {
    /// The co-located node's indexer cache directory. A relative path resolves
    /// against `--home`.
    pub cache_path: PathBuf,
    /// Base URL of the co-located node's httpd.
    pub node_url: String,
}

/// The GraphQL read API. `enabled = false` runs the indexer ingest-only.
#[derive(Debug, Clone, Deserialize)]
pub struct HttpdConfig {
    /// Serve the read API. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Socket address to bind, e.g. `0.0.0.0:8080`.
    #[serde(default = "default_bind")]
    pub bind: String,
}

impl Default for HttpdConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            bind: default_bind(),
        }
    }
}

fn default_max_connections() -> u32 {
    10
}

fn default_true() -> bool {
    true
}

fn default_bind() -> String {
    "0.0.0.0:8080".to_string()
}

#[cfg(test)]
mod tests {
    use {super::*, dango_config_parser::parse_config};

    /// The committed `config.example.toml` (also the user-facing example).
    fn example_config() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.example.toml")
    }

    /// The shipped example config parses into the expected shape (the
    /// `block_source` tag selects the right variant, the fetcher defaults /
    /// resolves, `max_connections` falls back to its default), and an
    /// `SECTION__FIELD` environment variable overrides the TOML — the
    /// `dango-cli` behaviour we must preserve. Doubling as a guard that the
    /// documented example always stays valid.
    ///
    /// Both halves live in one test on purpose: `parse_config` reads the whole
    /// process environment, so a separate env-mutating test could race a
    /// parallel one. Here the override is set and removed within a single,
    /// serial body.
    #[test]
    fn config_parses_and_env_overrides() {
        let path = example_config();

        // Baseline: TOML values + defaults.
        let cfg: Config = parse_config(&path).unwrap();
        assert_eq!(
            cfg.postgres.url,
            "postgres://postgres@localhost/indexer_historical"
        );
        assert_eq!(cfg.postgres.max_connections, 10); // default
        match &cfg.block_source {
            BlockSourceConfig::Remote(remote) => {
                assert_eq!(remote.store_path, PathBuf::from("data/blocks"));
                assert_eq!(remote.live_url, "http://node:8080");
                assert!(matches!(remote.fetcher, FetcherConfig::Sentinel));
            },
            other => panic!("expected a remote source, got {other:?}"),
        }
        assert!(cfg.httpd.enabled);
        assert_eq!(cfg.httpd.bind, "0.0.0.0:8080");

        // Environment override: `POSTGRES__URL` wins over the TOML value.
        // SAFETY: single-threaded within this test; set then immediately
        // cleared, and no other test reads this key.
        unsafe { std::env::set_var("POSTGRES__URL", "postgres://override@host/db") };
        let overridden: Config = parse_config(&path).unwrap();
        unsafe { std::env::remove_var("POSTGRES__URL") };
        assert_eq!(overridden.postgres.url, "postgres://override@host/db");
    }
}
