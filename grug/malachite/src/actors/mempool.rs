use {
    crate::{
        ActorResult,
        actors::network::{GossipNetworkMsg, NetworkActorRef, NetworkMsg},
        app::AppRef,
    },
    grug::{Hash256, Tx},
    malachitebft_test_mempool::Event as NetworkEvent,
    ractor::{Actor, ActorRef, RpcReplyPort, async_trait},
    std::{cmp::min, collections::VecDeque, sync::Arc},
    tracing::{error, info},
};

pub type MempoolMsg = Msg;
pub type MempoolActorRef = ActorRef<Msg>;

#[derive(Default)]
pub struct State {
    txs: VecDeque<Tx>,
}

pub enum Msg {
    NetworkEvent(Arc<NetworkEvent>),
    Add(Tx),
    Take {
        amount: usize,
        reply: RpcReplyPort<Vec<Tx>>,
    },
    Remove(Vec<Hash256>),
}

impl From<Arc<NetworkEvent>> for Msg {
    fn from(event: Arc<NetworkEvent>) -> Self {
        Self::NetworkEvent(event)
    }
}

pub struct Mempool {
    network: NetworkActorRef,
    app: AppRef,
}

impl Mempool {
    pub fn new(network: NetworkActorRef, app: AppRef) -> Self {
        Self { network, app }
    }

    fn handle_msg(&self, msg: Msg, state: &mut State) -> ActorResult<()> {
        match msg {
            Msg::NetworkEvent(event) => self.handle_network_event(&event, state)?,
            Msg::Add(tx) => {
                self.gossip_tx(tx.clone())?;
                self.add_txs(vec![tx], state);
            },
            Msg::Take { amount, reply } => self.take(state, amount, reply)?,
            Msg::Remove(tx_hashes) => self.remove(tx_hashes, state)?,
        }

        Ok(())
    }

    fn handle_network_event(&self, event: &NetworkEvent, state: &mut State) -> ActorResult<()> {
        match event {
            NetworkEvent::Message(.., network_msg) => {
                self.handle_network_msg(network_msg, state);
            },
            e => info!("Network event: {:?}", e),
        }

        Ok(())
    }

    fn handle_network_msg(&self, msg: &GossipNetworkMsg, state: &mut State) {
        match msg {
            GossipNetworkMsg::TransactionBatch(batch) => {
                // TODO: Actually MempoolTransactionBatch is in prost format
                let txs = todo!();

                self.add_txs(txs, state);
            },
        }
    }

    fn add_txs(&self, batch: Vec<Tx>, state: &mut State) {
        // TODO: The check can be done in parallel
        let checked_txs = batch.into_iter().filter_map(|tx| {
            self.app
                .check_tx(tx.clone())
                .ok()
                .and_then(|check| check.result.ok().map(|_| tx))
        });

        state.txs.extend(checked_txs);
    }

    fn take(
        &self,
        state: &mut State,
        mut amount: usize,
        reply: RpcReplyPort<Vec<Tx>>,
    ) -> ActorResult<()> {
        let mut txes = Vec::with_capacity(min(amount, state.txs.len()));

        while amount > 0 {
            if let Some(tx) = state.txs.pop_front() {
                txes.push(tx);
                amount -= 1;
            } else {
                break;
            }
        }

        reply.send(txes)?;

        Ok(())
    }

    fn remove(&self, tx_hashes: Vec<Hash256>, state: &mut State) -> ActorResult<()> {
        state
            .txs
            .retain(|tx| !tx_hashes.contains(&tx.tx_hash().unwrap()));

        Ok(())
    }

    fn gossip_tx(&self, tx: Tx) -> ActorResult<()> {
        // TODO: Actually MempoolTransactionBatch is in prost format
        let msg = todo!();

        self.network.cast(NetworkMsg::BroadcastMsg(msg))?;

        Ok(())
    }
}

#[async_trait]
impl Actor for Mempool {
    type Arguments = ();
    type Msg = Msg;
    type State = State;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> ActorResult<Self::State> {
        self.network.link(myself.get_cell());

        self.network
            .cast(NetworkMsg::Subscribe(Box::new(myself.clone())))?;

        Ok(State::default())
    }

    async fn handle(
        &self,
        myself: MempoolActorRef,
        msg: MempoolMsg,
        state: &mut State,
    ) -> ActorResult<()> {
        if let Err(e) = self.handle_msg(msg, state) {
            error!("Error handling message: {:?}", e);
        }

        Ok(())
    }
}
