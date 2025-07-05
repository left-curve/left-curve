use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub grug: GrugConfig,
    pub indexer: IndexerConfig,
    pub httpd: HttpdConfig,
    pub metrics_httpd: HttpdConfig,
    pub tendermint: TendermintConfig,
    pub transactions: TransactionsConfig,
    pub sentry: SentryConfig,
    pub log_level: String,
    pub log_format: LogFormat,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

#[derive(Serialize, Deserialize)]
pub struct GrugConfig {
    pub wasm_cache_capacity: usize,
    pub query_gas_limit: u64,
}

impl Default for GrugConfig {
    fn default() -> Self {
        Self {
            wasm_cache_capacity: 1000,
            query_gas_limit: 100_000_000,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct SentryConfig {
    pub enabled: bool,
    pub dsn: String,
    pub environment: String,
    pub sample_rate: f32,
    pub traces_sample_rate: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct IndexerConfig {
    pub enabled: bool,
    pub keep_blocks: bool,
    pub database: IndexerDatabaseConfig,
}

#[derive(Serialize, Deserialize)]
pub struct IndexerDatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl Default for IndexerDatabaseConfig {
    fn default() -> Self {
        IndexerDatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpdConfig {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub cors_allowed_origin: Option<String>,
}

impl Default for HttpdConfig {
    fn default() -> Self {
        HttpdConfig {
            enabled: false,
            ip: "127.0.0.1".to_string(),
            port: 0,
            cors_allowed_origin: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TendermintConfig {
    pub rpc_addr: String,
    pub abci_addr: String,
}

impl Default for TendermintConfig {
    fn default() -> Self {
        Self {
            rpc_addr: "http://localhost:26657".to_string(),
            abci_addr: "http://localhost:26658".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TransactionsConfig {
    pub chain_id: String,
    pub gas_adjustment: f64,
}

impl Default for TransactionsConfig {
    fn default() -> Self {
        Self {
            chain_id: "dango-1".to_string(),
            gas_adjustment: 1.4,
        }
    }
}
