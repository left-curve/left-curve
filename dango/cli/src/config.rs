use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub grug: GrugConfig,
    pub indexer: IndexerConfig,
    pub tendermint: TendermintConfig,
    pub transactions: TransactionsConfig,
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

#[derive(Serialize, Deserialize)]
pub struct IndexerConfig {
    pub enabled: bool,
    pub keep_blocks: bool,
    pub database_url: String,
    pub httpd: IndexerHttpdConfig,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_blocks: false,
            database_url: "postgres://localhost".to_string(),
            httpd: IndexerHttpdConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct IndexerHttpdConfig {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub cors_allowed_origin: Option<String>,
}

impl Default for IndexerHttpdConfig {
    fn default() -> Self {
        IndexerHttpdConfig {
            enabled: false,
            ip: "127.0.0.1".to_string(),
            port: 8080,
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
