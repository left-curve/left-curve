use {
    libp2p_identity::Keypair,
    malachitebft_engine::util::output_port::{OutputPort, OutputPortSubscriber},
    malachitebft_metrics::SharedRegistry,
    malachitebft_test_mempool::{
        Channel::Mempool, Event, PeerId, handle::CtrlHandle, types::MempoolTransactionBatch,
    },
    ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, async_trait},
    std::{collections::BTreeSet, sync::Arc},
    tokio::task::JoinHandle,
    tracing::error,
};

pub use malachitebft_test_mempool::{Config as MempoolConfig, NetworkMsg as GossipNetworkMsg};
pub type MempoolNetworkMsg = Msg;
pub type MempoolNetworkActorRef = ActorRef<Msg>;

pub struct Args {
    pub keypair: Keypair,
    pub config: MempoolConfig,
    pub metrics: SharedRegistry,
}

pub enum State {
    Stopped,
    Running {
        peers: BTreeSet<PeerId>,
        output_port: OutputPort<Arc<Event>>,
        ctrl_handle: CtrlHandle,
        recv_task: JoinHandle<()>,
    },
}

pub enum Msg {
    /// Subscribe to gossip events
    Subscribe(OutputPortSubscriber<Arc<Event>>),

    /// Broadcast a message to all peers
    BroadcastMsg(MempoolTransactionBatch),

    /// Request the number of connected peers
    GetState { reply: RpcReplyPort<usize> },

    // Internal message
    #[doc(hidden)]
    NewEvent(Event),
}

pub struct MempoolNetwork;

impl MempoolNetwork {
    pub async fn spawn(
        keypair: Keypair,
        config: MempoolConfig,
        metrics: SharedRegistry,
    ) -> Result<ActorRef<Msg>, ractor::SpawnErr> {
        let args = Args {
            keypair,
            config,
            metrics,
        };

        let (actor_ref, _) = Actor::spawn(None, Self, args).await?;
        Ok(actor_ref)
    }
}

#[async_trait]
impl Actor for MempoolNetwork {
    type Arguments = Args;
    type Msg = Msg;
    type State = State;

    async fn pre_start(
        &self,
        myself: ActorRef<Msg>,
        args: Args,
    ) -> Result<State, ActorProcessingErr> {
        let handle =
            malachitebft_test_mempool::spawn(args.keypair, args.config, args.metrics).await?;
        let (mut recv_handle, ctrl_handle) = handle.split();

        let recv_task = tokio::spawn(async move {
            while let Some(event) = recv_handle.recv().await {
                if let Err(e) = myself.cast(Msg::NewEvent(event)) {
                    error!("Actor has died, stopping gossip mempool: {e:?}");
                    break;
                }
            }
        });

        Ok(State::Running {
            peers: BTreeSet::new(),
            output_port: OutputPort::default(),
            ctrl_handle,
            recv_task,
        })
    }

    async fn post_start(
        &self,
        _myself: ActorRef<Msg>,
        _state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    #[tracing::instrument(name = "gossip.mempool", skip_all)]
    async fn handle(
        &self,
        _myself: ActorRef<Msg>,
        msg: Msg,
        state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        let State::Running {
            peers,
            output_port,
            ctrl_handle,
            ..
        } = state
        else {
            return Ok(());
        };

        match msg {
            Msg::Subscribe(subscriber) => subscriber.subscribe_to_port(output_port),

            Msg::BroadcastMsg(batch) => {
                match GossipNetworkMsg::TransactionBatch(batch).to_network_bytes() {
                    Ok(bytes) => {
                        ctrl_handle.broadcast(Mempool, bytes).await?;
                    },
                    Err(e) => {
                        error!("Failed to serialize transaction batch: {e}");
                    },
                }
            },

            Msg::NewEvent(event) => {
                match event {
                    Event::PeerConnected(peer_id) => {
                        peers.insert(peer_id);
                    },
                    Event::PeerDisconnected(peer_id) => {
                        peers.remove(&peer_id);
                    },
                    _ => {},
                }

                let event = Arc::new(event);
                output_port.send(event);
            },

            Msg::GetState { reply } => {
                let number_peers = match state {
                    State::Stopped => 0,
                    State::Running { peers, .. } => peers.len(),
                };

                reply.send(number_peers)?;
            },
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Msg>,
        state: &mut State,
    ) -> Result<(), ActorProcessingErr> {
        let state = std::mem::replace(state, State::Stopped);

        if let State::Running {
            ctrl_handle,
            recv_task,
            ..
        } = state
        {
            ctrl_handle.wait_shutdown().await?;
            recv_task.await?;
        }

        Ok(())
    }
}
