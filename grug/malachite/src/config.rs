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

    pub host: HostConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HostConfig {
    #[serde(with = "humantime_serde")]
    pub block_time: Duration,
    pub max_tx_bytes: ByteSize,
}

pub struct GenesisConfig {
    pub chain_id: String,
    pub block: BlockInfo,
    pub genesis_state: GenesisState,
}
