use {
    crate::{
        ActorResult,
        actors::network::{GossipNetworkMsg, MempoolNetworkActorRef, MempoolNetworkMsg},
        app::MempoolAppRef,
        types::RawTx,
    },
    grug::{BorshDeExt, BorshSerExt, CheckTxOutcome},
    grug_app::AppError,
    malachitebft_test_mempool::{Event as NetworkEvent, types::MempoolTransactionBatch},
    prost_types::Any,
    ractor::{Actor, ActorRef, RpcReplyPort, async_trait},
    std::{cmp::min, collections::VecDeque, sync::Arc},
    tracing::{Span, error, info},
};

pub type MempoolMsg = Msg;
pub type MempoolActorRef = ActorRef<Msg>;

#[derive(Default)]
pub struct State {
    txs: VecDeque<RawTx>,
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
            Msg::Add { tx, reply } => self.add_tx(tx, reply, state)?,
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

    fn handle_network_msg(&self, msg: &GossipNetworkMsg, state: &mut State) -> ActorResult<()> {
        match msg {
            GossipNetworkMsg::TransactionBatch(batch) => {
                // TODO: Actually MempoolTransactionBatch is in prost format
                let txs = batch
                    .transaction_batch
                    .value
                    .deserialize_borsh::<Vec<RawTx>>()?;

                self.add_batch(txs, state);

                Ok(())
            },
        }
    }

    fn add_tx(
        &self,
        tx: RawTx,
        reply: RpcReplyPort<Result<CheckTxOutcome, AppError>>,
        state: &mut State,
    ) -> ActorResult<()> {
        let check_tx_outcome = self.app.check_tx(&tx);

        if let Ok(CheckTxOutcome { result: Ok(_), .. }) = check_tx_outcome {
            self.gossip_tx(tx.clone())?;
            state.txs.push_back(tx);
        }

        reply.send(check_tx_outcome)?;

        Ok(())
    }

    fn add_batch(&self, batch: Vec<RawTx>, state: &mut State) {
        // TODO: The check can be done in parallel
        let checked_txs = batch.into_iter().filter_map(|tx| {
            self.app
                .check_tx(&tx)
                .ok()
                .and_then(|check| check.result.ok().map(|_| tx))
        });

        state.txs.extend(checked_txs);
    }

    fn take(
        &self,
        state: &mut State,
        mut amount: usize,
        reply: RpcReplyPort<Vec<RawTx>>,
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

    fn remove(&self, txs: Vec<RawTx>, state: &mut State) -> ActorResult<()> {
        state.txs.retain(|tx| !txs.contains(tx));

        Ok(())
    }

    fn gossip_tx(&self, tx: RawTx) -> ActorResult<()> {
        // TODO: Actually MempoolTransactionBatch is in prost format
        let tx = tx.to_borsh_vec()?;
        let msg = Any {
            type_url: "type.googleapis.com/malachite.MempoolTransactionBatch".to_string(),
            value: tx,
        };

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
