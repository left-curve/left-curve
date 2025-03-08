use {
    config::{Environment, File},
    serde::{Deserialize, Serialize},
    std::path::Path,
};

pub fn parse_config<P>(path: P) -> Result<Config, config::ConfigError>
where
    P: AsRef<Path>,
{
    config::Config::builder()
        .add_source(File::from(path.as_ref()))
        .add_source(Environment::default().separator("__"))
        .build()?
        .try_deserialize()
}

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
    pub postgres_url: String,
    pub httpd: IndexerHttpdConfig,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_blocks: false,
            postgres_url: "postgres://localhost".to_string(),
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, assertor::*};

    #[test]
    fn test_parse_config_file() {
        std::env::set_var("TENDERMINT__RPC_ADDR", "BAR");

        let cfg = parse_config("testdata/default_config.toml").unwrap();

        assert_that!(cfg.tendermint.abci_addr.as_str()).is_equal_to("http://localhost:26658");
        assert_that!(cfg.tendermint.rpc_addr.as_str()).is_equal_to("BAR");
    }
}
