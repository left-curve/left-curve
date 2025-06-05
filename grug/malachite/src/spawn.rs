use {
    crate::{config::Config, ctx},
    malachitebft_engine::node::Node,
    malachitebft_metrics::{Metrics as ConsensusMetrics, SharedRegistry},
    malachitebft_sync::Metrics as SyncMetrics,
};

pub fn spawn_actors(cfg: Config, private_key: ctx!(SigningScheme::PrivateKey)) {
    let registry = SharedRegistry::global().with_moniker(cfg.moniker.as_str());
    let consensus_metrics = ConsensusMetrics::register(&registry);
    let sync_metrics: SyncMetrics = SyncMetrics::register(&registry);
}
