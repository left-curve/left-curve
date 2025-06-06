use {
    crate::{
        ActorResult,
        actors::{
            MempoolActorRef, MempoolMsg,
            host::state::{BLOCKS, PARTS, ROUNDS, State},
        },
        app::{HostApp, HostAppRef},
        context::Context,
        ctx,
        types::{ProposalFin, ProposalInit, ProposalPart},
    },
    grug::{Hash256, Inner, Storage},
    grug_app::{App, Db},
    k256::sha2::{Digest, Sha256},
    malachitebft_app::{
        consensus::Role,
        streaming::{StreamContent, StreamMessage},
        types::LocallyProposedValue,
    },
    malachitebft_core_types::{Round, ValueOrigin},
    malachitebft_engine::{
        consensus::{ConsensusMsg, ConsensusRef},
        network::{NetworkMsg, NetworkRef},
    },
    ractor::{Actor, RpcReplyPort, async_trait},
    std::{sync::Arc, time::Duration},
    tracing::info,
};

pub type HostRef = malachitebft_engine::host::HostRef<Context>;
pub type HostMsg = malachitebft_engine::host::HostMsg<Context>;

pub struct Host {
    app: HostAppRef,
    mempool: MempoolActorRef,
    network: NetworkRef<Context>,
    validator_set: ctx!(ValidatorSet),
    private_key: ctx!(SigningScheme::PrivateKey),
    address: ctx!(Address),
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
        _: HostRef,
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
            } => {
                self.get_value(state, height, round, timeout, reply_to)
                    .await
            },
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
        network: NetworkRef<Context>,
        validator_set: ctx!(ValidatorSet),
        private_key: ctx!(SigningScheme::PrivateKey),
    ) -> HostRef
    where
        DB: Db,
        App<DB, VM, PP, ID>: HostApp,
    {
        let args = State::new(app.db.consensus());

        let host = Host {
            app,
            mempool,
            network,
            validator_set,
            address: private_key.derive_address(),
            private_key,
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

    /// Equivalent of prepare_proposal
    async fn get_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        round: Round,
        _timeout: Duration,
        reply_to: RpcReplyPort<LocallyProposedValue<Context>>,
    ) -> ActorResult<()> {
        // If we have already built a block for this height and round, return it to consensus
        // This may happen when we are restarting after a crash and replaying the WAL.
        if let Some(value) = ROUNDS
            .idx
            .proposer
            .may_load_value(state, (*height, round.as_i64(), self.address))?
        {
            info!(%height, %round, hash = ?value.value, "Returning previously built value");

            reply_to.send(LocallyProposedValue::new(
                value.height,
                value.round,
                value.value,
            ))?;

            return Ok(());
        }

        // Crate proposal parts
        let (parts, hash) = {
            let mut parts = vec![];

            let part = ProposalPart::Init(ProposalInit::new(height, round, self.address));
            parts.push(part);

            // Take txs from mempool
            let txs = self
                .mempool
                .call(
                    |reply| MempoolMsg::Take {
                        // TODO: set a limit
                        amount: usize::MAX,
                        reply,
                    },
                    None,
                )
                .await?
                .success_or(anyhow::anyhow!("Failed to take txs from mempool"))?;

            let mut hasher = Sha256::new();

            // Call prepare_proposal
            let txs = self.app.prepare_proposal(txs);

            for tx in &txs {
                hasher.update(tx.as_ref());
            }

            parts.push(ProposalPart::Data(txs));

            let hash = hasher.finalize();
            let sig = self.private_key.sign_digest(hash);

            parts.push(ProposalPart::Fin(ProposalFin::new(hash, sig)));

            (parts, hash)
        };

        // Store parts
        let stream_id = state.stream_id();
        PARTS.save(state, &stream_id.to_bytes(), &parts)?;

        // Stream parts to consensus
        {
            let mut sequence = 0;

            for part in parts {
                let msg =
                    StreamMessage::new(stream_id.clone(), sequence, StreamContent::Data(part));
                self.network.cast(NetworkMsg::PublishProposalPart(msg))?;
                sequence += 1;
            }

            let msg = StreamMessage::new(stream_id.clone(), sequence, StreamContent::Fin);
            self.network.cast(NetworkMsg::PublishProposalPart(msg))?;
        }

        // Return the proposed value
        {
            let value = LocallyProposedValue::new(height, round, hash.into());
            reply_to.send(value)?;
        }

        Ok(())
    }
}
