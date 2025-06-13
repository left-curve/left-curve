use {
    crate::{
        ActorResult,
        actors::network::{GossipNetworkMsg, MempoolNetworkActorRef, MempoolNetworkMsg},
        app::MempoolAppRef,
        types::RawTx,
    },
    grug::{BorshDeExt, BorshSerExt, CheckTxOutcome, Hash256},
    grug_app::AppError,
    malachitebft_test_mempool::{Event as NetworkEvent, types::MempoolTransactionBatch},
    prost_types::Any,
    ractor::{Actor, ActorRef, RpcReplyPort, async_trait},
    std::{
        cmp::min,
        collections::{HashMap, HashSet, VecDeque},
        sync::Arc,
    },
    tracing::{Span, error, info, warn},
};

pub type MempoolMsg = Msg;
pub type MempoolActorRef = ActorRef<Msg>;

#[derive(Default)]
pub struct State {
    txs: VecDeque<RawTx>,
    tx_hashes: HashMap<Hash256, usize>,
}

impl State {
    pub fn exists(&self, tx: Hash256) -> bool {
        self.tx_hashes.get(&tx).is_some()
    }
}

pub enum Msg {
    NetworkEvent(Arc<NetworkEvent>),
    Add {
        tx: RawTx,
        reply: RpcReplyPort<Result<CheckTxOutcome, AppError>>,
    },
    Take {
        amount: usize,
        reply: RpcReplyPort<Vec<RawTx>>,
    },
    Remove(Vec<RawTx>),
}

impl From<Arc<NetworkEvent>> for Msg {
    fn from(event: Arc<NetworkEvent>) -> Self {
        Self::NetworkEvent(event)
    }
}

pub struct Mempool {
    mempool_network: MempoolNetworkActorRef,
    app: MempoolAppRef,
    span: Span,
}

impl Mempool {
    pub async fn spawn(
        mempool_network: MempoolNetworkActorRef,
        app: MempoolAppRef,
        span: Span,
    ) -> Result<MempoolActorRef, ractor::SpawnErr> {
        let node = Self {
            mempool_network,
            app,
            span,
        };

        let (actor_ref, _) = Actor::spawn(None, node, ()).await?;
        Ok(actor_ref)
    }

    fn handle_msg(&self, msg: Msg, state: &mut State) -> ActorResult<()> {
        match msg {
            Msg::NetworkEvent(event) => self.handle_network_event(&event, state)?,
            Msg::Add { tx, reply } => self.add_tx(tx, Some(reply), state)?,
            Msg::Take { amount, reply } => self.take(state, amount, reply)?,
            Msg::Remove(tx_hashes) => self.remove(tx_hashes, state)?,
        }

        Ok(())
    }

    fn handle_network_event(&self, event: &NetworkEvent, state: &mut State) -> ActorResult<()> {
        match event {
            NetworkEvent::Message(.., network_msg) => {
                self.handle_network_msg(network_msg, state)?;
            },
            e => info!("Network event: {:?}", e),
        }

        Ok(())
    }

    #[tracing::instrument("handle_network_msg", skip_all)]
    fn handle_network_msg(&self, msg: &GossipNetworkMsg, state: &mut State) -> ActorResult<()> {
        match msg {
            GossipNetworkMsg::TransactionBatch(batch) => {
                // TODO: Actually MempoolTransactionBatch is in prost format
                let txs = batch.transaction_batch.value.deserialize_borsh::<RawTx>()?;

                self.add_tx(txs, None, state)
            },
        }
    }

    #[tracing::instrument("add_tx", skip_all)]
    fn add_tx(
        &self,
        tx: RawTx,
        reply: Option<RpcReplyPort<Result<CheckTxOutcome, AppError>>>,
        state: &mut State,
    ) -> ActorResult<()> {
        let tx_hash = tx.hash();

        if state.exists(tx_hash) {
            warn!("tx already exists in mempool");
            return Ok(());
        }

        let check_tx_outcome = self.app.check_tx(&tx);

        if let Ok(CheckTxOutcome { result: Ok(_), .. }) = check_tx_outcome {
            info!("tx added to mempool");
            state.tx_hashes.insert(tx_hash, state.txs.len());
            state.txs.push_back(tx.clone());
            self.gossip_tx(tx)?;
        } else {
            warn!(reason = ?check_tx_outcome, "check_tx failed!");
        }

        if let Some(reply) = reply {
            reply.send(check_tx_outcome)?;
        }

        Ok(())
    }

    fn take(
        &self,
        state: &mut State,
        mut amount: usize,
        reply: RpcReplyPort<Vec<RawTx>>,
    ) -> ActorResult<()> {
        let mut txs = Vec::with_capacity(min(amount, state.txs.len()));

        if amount == 0 {
            reply.send(txs)?;
            return Ok(());
        }

        for tx in state.txs.iter() {
            // we are not removing the tx from the mempool, here because prepare proposal could not
            // include some txs. Txs will be removed during decided.
            txs.push(tx.clone());
            amount -= 1;
            if amount == 0 {
                break;
            }
        }

        reply.send(txs)?;

        Ok(())
    }

    #[tracing::instrument("remove", skip_all)]
    fn remove(&self, txs: Vec<RawTx>, state: &mut State) -> ActorResult<()> {
        let mut ignore = HashSet::new();
        for tx in txs {
            if let Some(index) = state.tx_hashes.remove(&tx.hash()) {
                ignore.insert(index);
            }
        }

        info!("removed {} txs from mempool", ignore.len());

        let mut new_txs = VecDeque::with_capacity(state.txs.len() - ignore.len());
        let mut new_hashes = HashMap::with_capacity(state.txs.len() - ignore.len());

        let mut counter = 0;
        for (index, tx) in state.txs.iter().enumerate() {
            if !ignore.contains(&index) {
                new_hashes.insert(tx.hash(), counter);
                new_txs.push_back(tx.clone());
                counter += 1;
            }
        }

        state.txs = new_txs;
        state.tx_hashes = new_hashes;

        Ok(())
    }

    #[tracing::instrument("gossip_tx", skip_all)]
    fn gossip_tx(&self, tx: RawTx) -> ActorResult<()> {
        // TODO: Actually MempoolTransactionBatch is in prost format
        let tx = tx.to_borsh_vec()?;
        let msg = Any {
            type_url: "type.googleapis.com/malachite.MempoolTransactionBatch".to_string(),
            value: tx,
        };

        info!("gossiping tx");

        self.mempool_network
            .cast(MempoolNetworkMsg::Broadcast(MempoolTransactionBatch::new(
                msg,
            )))?;

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
        self.mempool_network.link(myself.get_cell());

        self.mempool_network
            .cast(MempoolNetworkMsg::Subscribe(Box::new(myself.clone())))?;

        Ok(State::default())
    }

    #[tracing::instrument("mempool", parent = &self.span, skip_all)]
    async fn handle(
        &self,
        _myself: MempoolActorRef,
        msg: MempoolMsg,
        state: &mut State,
    ) -> ActorResult<()> {
        if let Err(e) = self.handle_msg(msg, state) {
            error!("Error handling message: {:?}", e);
        }

        Ok(())
    }
}
