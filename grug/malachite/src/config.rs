use {
    bytesize::ByteSize,
    grug::{BlockInfo, GenesisState},
    malachitebft_app::config::{
        ConsensusConfig, LoggingConfig, MempoolConfig, MetricsConfig, RuntimeConfig,
        ValueSyncConfig,
    },
    serde::{Deserialize, Serialize},
    std::time::Duration,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorsConfig {
    /// A custom human-readable name for this node
    pub moniker: String,

    /// Log configuration options
    pub logging: LoggingConfig,

    /// Consensus configuration options
    pub consensus: ConsensusConfig,

    /// Mempool configuration options
    pub mempool: MempoolConfig,

    /// Sync configuration options
    pub value_sync: ValueSyncConfig,

    /// Metrics configuration options
    pub metrics: MetricsConfig,

    /// Runtime configuration options
    pub runtime: RuntimeConfig,

    /// Host configuration options
    pub host: HostConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostConfig {
    #[serde(with = "humantime_serde")]
    pub block_time: Duration,
    pub max_tx_bytes: ByteSize,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            block_time: Duration::from_millis(500),
            max_tx_bytes: ByteSize::mb(4),
        }
    }
}

pub struct GenesisConfig {
    pub chain_id: String,
    pub block: BlockInfo,
    pub genesis_state: GenesisState,
}
