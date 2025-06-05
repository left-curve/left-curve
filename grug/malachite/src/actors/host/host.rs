use {
    crate::{
        ActorResult,
        actors::{
            MempoolActorRef,
            host::state::{BLOCKS, ROUNDS, State},
        },
        app::{HostApp, HostAppRef},
        context::Context,
        ctx,
    },
    grug::Storage,
    grug_app::{App, Db},
    malachitebft_app::consensus::Role,
    malachitebft_core_types::{Round, ValueOrigin},
    malachitebft_engine::consensus::{ConsensusMsg, ConsensusRef},
    ractor::{Actor, async_trait},
    std::sync::Arc,
    tracing::info,
};

pub type HostRef = malachitebft_engine::host::HostRef<Context>;
pub type HostMsg = malachitebft_engine::host::HostMsg<Context>;

pub struct Host {
    app: HostAppRef,
    mempool: MempoolActorRef,
    validator_set: ctx!(ValidatorSet),
}

#[async_trait]
impl Actor for Host {
    type Arguments = State;
    type Msg = HostMsg;
    type State = State;

    async fn pre_start(&self, myself: HostRef, args: Self::Arguments) -> ActorResult<Self::State> {
        self.mempool.link(myself.get_cell());

        Ok(args)
    }

    async fn handle(
        &self,
        myself: HostRef,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> ActorResult<()> {
        match message {
            HostMsg::ConsensusReady(actor_ref) => self.consensus_ready(state, actor_ref),
            HostMsg::StartedRound {
                height,
                round,
                proposer,
                role,
            } => self.started_round(state, height, round, proposer, role),
            HostMsg::GetValue {
                height,
                round,
                timeout,
                reply_to,
            } => todo!(),
            HostMsg::ExtendVote {
                height,
                round,
                value_id,
                reply_to,
            } => todo!(),
            HostMsg::VerifyVoteExtension {
                height,
                round,
                value_id,
                extension,
                reply_to,
            } => todo!(),
            HostMsg::RestreamValue {
                height,
                round,
                valid_round,
                address,
                value_id,
            } => todo!(),
            HostMsg::GetHistoryMinHeight { reply_to } => todo!(),
            HostMsg::ReceivedProposalPart {
                from,
                part,
                reply_to,
            } => todo!(),
            HostMsg::GetValidatorSet { height, reply_to } => todo!(),
            HostMsg::Decided {
                certificate,
                extensions,
                consensus,
            } => todo!(),
            HostMsg::GetDecidedValue { height, reply_to } => todo!(),
            HostMsg::ProcessSyncedValue {
                height,
                round,
                validator_address,
                value_bytes,
                reply_to,
            } => todo!(),
        }
    }
}

impl Host {
    pub async fn spawn<DB, VM, PP, ID>(
        app: Arc<App<DB, VM, PP, ID>>,
        mempool: MempoolActorRef,
        validator_set: ctx!(ValidatorSet),
    ) -> HostRef
    where
        DB: Db,
        App<DB, VM, PP, ID>: HostApp,
    {
        let args = State::new(app.db.consensus());

        let host = Host {
            app,
            mempool,
            validator_set,
        };

        let (actor_ref, _) = Actor::spawn(None, host, args).await.unwrap();
        actor_ref
    }

    fn consensus_ready(
        &self,
        storage: &mut dyn Storage,
        consensus: ConsensusRef<Context>,
    ) -> ActorResult<()> {
        let height = BLOCKS
            .keys(storage, None, None, grug::Order::Descending)
            .next()
            .transpose()?
            .unwrap_or_default();

        // TODO: is a sleep necessary here? It's present in the example implementation

        consensus.cast(ConsensusMsg::StartHeight(
            <ctx!(Height)>::new(height + 1),
            self.validator_set.clone(),
        ))?;

        Ok(())
    }

    fn started_round(
        &self,
        state: &mut State,
        height: ctx!(Height),
        round: Round,
        proposer: ctx!(Address),
        role: Role,
    ) -> ActorResult<()> {
        state.height = height;
        state.round = round;
        state.proposer = Some(proposer);
        state.role = role;

        // If we have already built or seen one or more values for this height and round,
        // feed them back to consensus. This may happen when we are restarting after a crash.

        let consensus = state.consensus()?;

        for value in ROUNDS.prefix(*height).append(round.as_i64()).values(
            state,
            None,
            None,
            grug::Order::Ascending,
        ) {
            let value = value?;
            info!(%height, %round, hash = ?value.value, "Replaying already known proposed value");

            consensus.cast(ConsensusMsg::ReceivedProposedValue(
                value,
                ValueOrigin::Consensus,
            ))?;
        }

        Ok(())
    }
}
