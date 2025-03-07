use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub indexer_httpd: IndexerHttpdConfig,
    pub indexer: IndexerConfig,
}

#[derive(Deserialize, Default)]
pub struct IndexerConfig {
    pub enabled: bool,
    pub keep_blocks: bool,
    #[serde(default = "default_database_url")]
    pub database_url: String,
}

fn default_database_url() -> String {
    "postgres://localhost".to_string()
}

#[derive(Deserialize, Default)]
pub struct IndexerHttpdConfig {
    pub enabled: bool,
    #[serde(default = "default_tendermint_endpoint")]
    pub tendermint_endpoint: String,
}

fn default_tendermint_endpoint() -> String {
    "http://localhost:26657".to_string()
}
