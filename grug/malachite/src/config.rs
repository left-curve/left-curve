use malachitebft_app::config::{
    ConsensusConfig, LoggingConfig, MempoolConfig, MetricsConfig, RuntimeConfig, ValueSyncConfig,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
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
}
