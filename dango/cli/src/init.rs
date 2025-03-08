use {crate::home_directory::HomeDirectory, anyhow::ensure, clap::Parser};

/// The default configurations as a TOML string.
///
/// Unfortunately, Rust serde doesn't support including doc strings when
/// serializing. If we want to include doc strings for the best UX, using a
/// string literal seems to be the only option.
const DEFAULT_CONFIG_TOML: &str = r#"# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

################################################################################
###                            Grug Configuration                            ###
################################################################################

[grug]

# Capacity of the wasm module cache; zero means to not use a cache.
wasm_cache_capacity = 1000

# Gas limit when serving query requests.
query_gas_limit = 100000000

################################################################################
###                          Indexer Configuration                           ###
################################################################################

[indexer]

# Whether to enable indexer.
enabled = false

# Whether to store block respnonses.
keep_blocks = false

# URL to the PostgreSQL database.
postgres_url = "postgres://localhost"

[indexer.httpd]

# Whether to enable the HTTP server.
enabled = false

# IP address to listen on. `0.0.0.0` means all interfaces.
ip = "0.0.0.0"

# Port to listen on.
port = 8080

# Allowed origin for CORS.
cors_allowed_origin = "*"

################################################################################
###                         Tendermint Configuration                         ###
################################################################################

[tendermint]

# Tendermint RPC listening address.
rpc_addr = "http://localhost:26657"

# Tendermint ABCI listening address.
abci_addr = "http://localhost:26658"

################################################################################
###                        Transaction Configuration                         ###
################################################################################

[transactions]

# Chain identifier.
chain_id = "dango-1"

# Scaling factor to apply to simulated gas consumption.
gas_adjustment = 1.4
"#;

#[derive(Parser)]
pub struct InitCmd;

impl InitCmd {
    pub fn run(&self, home: &HomeDirectory) -> anyhow::Result<()> {
        ensure!(
            !home.exists(),
            "home directory already exists: {}",
            home.as_os_str().to_str().unwrap()
        );

        std::fs::create_dir_all(home)?;
        std::fs::create_dir(home.data_dir())?;
        std::fs::create_dir(home.keys_dir())?;
        std::fs::create_dir(home.indexer_dir())?;
        std::fs::write(home.config_file(), DEFAULT_CONFIG_TOML)?;

        tracing::info!(
            "Dango directory initiated at: {}",
            home.as_os_str().to_str().unwrap()
        );

        Ok(())
    }
}
