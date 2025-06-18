use {
    crate::{
        GenesisConfig, HostConfig,
        app::{HostApp, MempoolApp},
        codec,
        config::ActorsConfig,
        context::Context,
        ctx,
        host::{Host, HostRef, latest_height},
        mempool::{Mempool, MempoolActorRef},
        network::{MempoolNetwork, MempoolNetworkActorRef},
    },
    grug_app::{App, AppError, Db, Indexer, LAST_FINALIZED_BLOCK, ProposalPreparer, Vm},
    malachitebft_app::events::TxEvent,
    malachitebft_config::{
        BootstrapProtocol, ConsensusConfig, PubSubProtocol, Selector, ValuePayload, ValueSyncConfig,
    },
    malachitebft_engine::{
        consensus::{Consensus, ConsensusParams, ConsensusRef},
        network::{Network, NetworkRef},
        node::{Node, NodeRef},
        sync::{Params as SyncParams, Sync as SyncActor, SyncRef},
        wal::{Wal, WalRef},
    },
    malachitebft_metrics::{Metrics as ConsensusMetrics, SharedRegistry},
    malachitebft_sync::Metrics as SyncMetrics,
    std::{fmt::Debug, path::PathBuf, sync::Arc, time::Duration},
    tokio::task::JoinHandle,
    tracing::Span,
};

pub struct Actors {
    pub mempool: MempoolActorRef,
    pub node: NodeRef,
    pub node_handle: JoinHandle<()>,
}

pub async fn spawn_actors<DB, VM, PP, ID>(
    home_dir: Option<PathBuf>,
    cfg: ActorsConfig,
    validator_set: ctx!(ValidatorSet),
    private_key: ctx!(SigningScheme::PrivateKey),
    app: Arc<App<DB, VM, PP, ID>>,
    genesis_config: Option<GenesisConfig>,
    span: Option<Span>,
) -> Actors
where
    VM: Vm + Clone + Send + Sync,
    PP: ProposalPreparer,
    ID: Indexer,
    DB: Db,
    DB::Error: Debug,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
    App<DB, VM, PP, ID>: MempoolApp,
    App<DB, VM, PP, ID>: HostApp,
{
    let start_height = {
        let app_storage = app.db.state_storage(None).unwrap();
        let consensus_storage = app.db.consensus();
        let consensus_height = latest_height(&consensus_storage).unwrap_or(0);

        match LAST_FINALIZED_BLOCK.may_load(&app_storage).unwrap() {
            // Genesis is done
            Some(block) => {
                if block.height != consensus_height {
                    panic!("Consensus height is not the same as the app height");
                }

                block.height + 1
            },
            // Genesis is not done
            None => {
                if consensus_height != 0 {
                    panic!("Genesis has not been committed, but the consensus height is not 0");
                }

                let genesis_config = genesis_config
                    .expect("Genesis need to be runned, but genesis config is not provided");

                // run the genesis block
                app.do_init_chain(
                    genesis_config.chain_id,
                    genesis_config.block,
                    genesis_config.genesis_state,
                )
                .unwrap();

                // Sanity check:
                assert_eq!(
                    LAST_FINALIZED_BLOCK.load(&app_storage).expect("Genesis has been just committed, but LAST_FINALIZED_BLOCK can't be loaded").height,
                    0,
                    "Genesis block height is not 0"
                );

                1
            },
        }
    };

    let span = span.unwrap_or(Span::current());

    let registry = SharedRegistry::global().with_moniker(cfg.moniker.as_str());

    let consensus_metrics = ConsensusMetrics::register(&registry);
    let sync_metrics: SyncMetrics = SyncMetrics::register(&registry);

    let mempool_network = spawn_mempool_network_actor(&cfg, &private_key, &registry).await;
    let mempool = spawn_mempool_actor(mempool_network, app.clone(), span.clone()).await;

    let network = spawn_network_actor(&cfg, &private_key, &registry, &span).await;

    let host = spawn_host_actor(
        private_key.derive_address(),
        app,
        cfg.host,
        mempool.clone(),
        network.clone(),
        validator_set.clone(),
        span.clone(),
    )
    .await;

    let sync = spawn_sync_actor(
        network.clone(),
        host.clone(),
        &cfg.value_sync,
        sync_metrics,
        &span,
    )
    .await;

    let wal = spawn_wal_actor(home_dir, &registry, &span).await;

    let consensus = spawn_consensus_actor(
        <ctx!(Height)>::new(start_height),
        validator_set,
        cfg.consensus,
        private_key,
        network.clone(),
        host.clone(),
        wal.clone(),
        sync.clone(),
        consensus_metrics,
        TxEvent::new(),
        &span,
    )
    .await;

    let node = Node::new(Context, network, consensus, wal, sync, host, span);

    let (actor_ref, handle) = node.spawn().await.unwrap();

    Actors {
        mempool,
        node: actor_ref,
        node_handle: handle,
    }
}

async fn spawn_mempool_actor(
    mempool_network: MempoolNetworkActorRef,
    app: Arc<dyn MempoolApp>,
    span: Span,
) -> MempoolActorRef {
    Mempool::spawn(mempool_network, app, span).await.unwrap()
}

async fn spawn_mempool_network_actor(
    cfg: &ActorsConfig,
    private_key: &ctx!(SigningScheme::PrivateKey),
    registry: &SharedRegistry,
) -> MempoolNetworkActorRef {
    let config = malachitebft_test_mempool::Config {
        listen_addr: cfg.mempool.p2p.listen_addr.clone(),
        persistent_peers: cfg.mempool.p2p.persistent_peers.clone(),
        idle_connection_timeout: Duration::from_secs(15 * 60),
    };

    MempoolNetwork::spawn(private_key.to_keypair().into(), config, registry.clone())
        .await
        .unwrap()
}

async fn spawn_network_actor(
    cfg: &ActorsConfig,
    private_key: &ctx!(SigningScheme::PrivateKey),
    registry: &SharedRegistry,
    span: &tracing::Span,
) -> NetworkRef<Context> {
    use malachitebft_network as gossip;

    let bootstrap_protocol = match cfg.consensus.p2p.discovery.bootstrap_protocol {
        BootstrapProtocol::Kademlia => gossip::BootstrapProtocol::Kademlia,
        BootstrapProtocol::Full => gossip::BootstrapProtocol::Full,
    };

    let selector = match cfg.consensus.p2p.discovery.selector {
        Selector::Kademlia => gossip::Selector::Kademlia,
        Selector::Random => gossip::Selector::Random,
    };

    let config_gossip = gossip::Config {
        listen_addr: cfg.consensus.p2p.listen_addr.clone(),
        persistent_peers: cfg.consensus.p2p.persistent_peers.clone(),
        discovery: gossip::DiscoveryConfig {
            enabled: cfg.consensus.p2p.discovery.enabled,
            bootstrap_protocol,
            selector,
            num_outbound_peers: cfg.consensus.p2p.discovery.num_outbound_peers,
            num_inbound_peers: cfg.consensus.p2p.discovery.num_inbound_peers,
            ephemeral_connection_timeout: cfg.consensus.p2p.discovery.ephemeral_connection_timeout,
            ..Default::default()
        },
        idle_connection_timeout: Duration::from_secs(15 * 60),
        transport: gossip::TransportProtocol::from_multiaddr(&cfg.consensus.p2p.listen_addr)
            .unwrap_or_else(|| {
                panic!(
                    "No valid transport protocol found in listen address: {}",
                    cfg.consensus.p2p.listen_addr
                )
            }),
        pubsub_protocol: match cfg.consensus.p2p.protocol {
            PubSubProtocol::GossipSub(_) => gossip::PubSubProtocol::GossipSub,
            PubSubProtocol::Broadcast => gossip::PubSubProtocol::Broadcast,
        },
        gossipsub: match cfg.consensus.p2p.protocol {
            PubSubProtocol::GossipSub(config) => gossip::GossipSubConfig {
                mesh_n: config.mesh_n(),
                mesh_n_high: config.mesh_n_high(),
                mesh_n_low: config.mesh_n_low(),
                mesh_outbound_min: config.mesh_outbound_min(),
            },
            PubSubProtocol::Broadcast => gossip::GossipSubConfig::default(),
        },
        rpc_max_size: cfg.consensus.p2p.rpc_max_size.as_u64() as usize,
        pubsub_max_size: cfg.consensus.p2p.pubsub_max_size.as_u64() as usize,
        enable_sync: true,
    };

    Network::spawn(
        private_key.to_keypair().into(),
        config_gossip,
        registry.clone(),
        codec::Borsh,
        span.clone(),
    )
    .await
    .unwrap()
}

async fn spawn_host_actor<DB, VM, PP, ID>(
    address: ctx!(Address),
    app: Arc<App<DB, VM, PP, ID>>,
    config: HostConfig,
    mempool: MempoolActorRef,
    network: NetworkRef<Context>,
    validator_set: ctx!(ValidatorSet),
    span: Span,
) -> HostRef
where
    DB: Db,
    App<DB, VM, PP, ID>: HostApp,
{
    Host::spawn(address, app, mempool, network, validator_set, span, config).await
}

async fn spawn_sync_actor(
    network: NetworkRef<Context>,
    host: HostRef,
    config: &ValueSyncConfig,
    sync_metrics: SyncMetrics,
    span: &Span,
) -> Option<SyncRef<Context>> {
    if !config.enabled {
        return None;
    }

    let params = SyncParams {
        status_update_interval: config.status_update_interval,
        request_timeout: config.request_timeout,
    };

    let actor_ref = SyncActor::spawn(Context, network, host, params, sync_metrics, span.clone())
        .await
        .unwrap();

    Some(actor_ref)
}

async fn spawn_wal_actor(
    home_dir: Option<PathBuf>,
    registry: &SharedRegistry,
    span: &tracing::Span,
) -> WalRef<Context> {
    let wal_dir = home_dir.unwrap_or_else(|| PathBuf::from(".")).join("wal");
    std::fs::create_dir_all(&wal_dir).unwrap();
    let wal_file = wal_dir.join("consensus.wal");

    Wal::spawn(
        &Context,
        codec::Borsh,
        wal_file,
        registry.clone(),
        span.clone(),
    )
    .await
    .unwrap()
}

async fn spawn_consensus_actor(
    initial_height: ctx!(Height),
    initial_validator_set: ctx!(ValidatorSet),
    cfg: ConsensusConfig,
    signing_provider: ctx!(SigningScheme::PrivateKey),
    network: NetworkRef<Context>,
    host: HostRef,
    wal: WalRef<Context>,
    sync: Option<SyncRef<Context>>,
    consensus_metrics: ConsensusMetrics,
    tx_event: TxEvent<Context>,
    span: &tracing::Span,
) -> ConsensusRef<Context> {
    let consensus_params = ConsensusParams {
        initial_height,
        initial_validator_set,
        address: signing_provider.derive_address(),
        threshold_params: Default::default(),
        value_payload: match cfg.value_payload {
            ValuePayload::PartsOnly => malachitebft_core_types::ValuePayload::PartsOnly,
            ValuePayload::ProposalAndParts => {
                malachitebft_core_types::ValuePayload::ProposalAndParts
            },
            ValuePayload::ProposalOnly => {
                panic!("ProposalOnly is not supported for actor-app-with-parts")
            },
        },
    };

    Consensus::spawn(
        Context,
        consensus_params,
        cfg.timeouts,
        Box::new(signing_provider),
        network,
        host,
        wal,
        sync,
        consensus_metrics,
        tx_event,
        span.clone(),
    )
    .await
    .unwrap()
}
