use {
    bytesize::ByteSize,
    grug::{BlockInfo, GenesisState},
    malachitebft_app::config::{
        ConsensusConfig, LoggingConfig, MetricsConfig, RuntimeConfig, ValueSyncConfig,
    },
    malachitebft_config::P2pConfig,
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
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            block_time: Duration::from_millis(500),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// P2P configuration options
    pub p2p: P2pConfig,
    pub max_txs_bytes: ByteSize,
    pub avg_tx_bytes: ByteSize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            p2p: P2pConfig::default(),
            max_txs_bytes: ByteSize::mb(4),
            avg_tx_bytes: ByteSize::kb(100),
        }
    }
}

pub struct GenesisConfig {
    pub chain_id: String,
    pub block: BlockInfo,
    pub genesis_state: GenesisState,
}
