use {
    crate::{
        ActorResult,
        actors::{
            MempoolActorRef, MempoolMsg,
            host::state::{State, StoreParts},
        },
        app::{HostApp, HostAppRef},
        context::Context,
        ctx,
        types::{ProposalFin, ProposalInit},
    },
    grug::Inner,
    grug_app::{App, Db},
    k256::sha2::{Digest, Sha256},
    malachitebft_app::{
        consensus::Role,
        streaming::{StreamContent, StreamId, StreamMessage},
        types::{LocallyProposedValue, ProposedValue},
    },
    malachitebft_core_types::{Round, ValueOrigin},
    malachitebft_engine::{
        consensus::{ConsensusMsg, ConsensusRef},
        network::{NetworkMsg, NetworkRef},
    },
    malachitebft_sync::PeerId,
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
            HostMsg::GetHistoryMinHeight { reply_to } => {
                self.get_history_min_height(state, reply_to)
            },
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
            HostMsg::RestreamValue {
                height,
                round,
                valid_round,
                address,
                value_id,
            } => {
                self.restream_value(state, height, round, valid_round, address, value_id)
                    .await
            },
            HostMsg::ReceivedProposalPart {
                from,
                part,
                reply_to,
            } => {
                self.received_proposal_part(state, from, part, reply_to)
                    .await
            },
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
            HostMsg::ExtendVote { reply_to, .. } => {
                reply_to.send(None)?;
                Ok(())
            },
            HostMsg::VerifyVoteExtension { reply_to, .. } => {
                reply_to.send(Ok(()))?;
                Ok(())
            },
        }
    }
}

//  ----------------------------- Generic methods ------------------------------

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

    pub async fn stream_parts(
        &self,
        stream_id: StreamId,
        parts: StoreParts,
        with_fin: bool,
    ) -> ActorResult<()> {
        let mut sequence = 0;

        for part in parts {
            let msg = StreamMessage::new(stream_id.clone(), sequence, StreamContent::Data(part));
            self.network.cast(NetworkMsg::PublishProposalPart(msg))?;
            sequence += 1;
        }

        if with_fin {
            let msg = StreamMessage::new(stream_id.clone(), sequence, StreamContent::Fin);
            self.network.cast(NetworkMsg::PublishProposalPart(msg))?;
        }

        Ok(())
    }
}

// ------------------------------- Msgs handlers -------------------------------

impl Host {
    fn consensus_ready(
        &self,
        state: &mut State,
        consensus: ConsensusRef<Context>,
    ) -> ActorResult<()> {
        let height = state
            .with_db_storage(|storage, db| {
                db.blocks
                    .keys(storage, None, None, grug::Order::Descending)
                    .next()
                    .transpose()
            })?
            .unwrap_or_default();

        state.set_consensus(consensus.clone());

        // TODO: is a sleep necessary here? It's present in the example implementation

        consensus.cast(ConsensusMsg::StartHeight(
            <ctx!(Height)>::new(height + 1),
            self.validator_set.clone(),
        ))?;

        Ok(())
    }

    fn get_history_min_height(
        &self,
        state: &State,
        reply_to: RpcReplyPort<ctx!(Height)>,
    ) -> ActorResult<()> {
        let height = state
            .with_db_storage(|storage, db| {
                db.blocks
                    .keys(storage, None, None, grug::Order::Ascending)
                    .next()
                    .transpose()
            })?
            .unwrap_or_default();

        reply_to.send(<ctx!(Height)>::new(height))?;

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

        state.with_db_storage(|storage, db| {
            for value in db.undecided_proposals
                .prefix(*height)
                .append(round.as_i64())
                .values(storage, None, None, grug::Order::Ascending) {
                    let value = value?;
                    info!(%height, %round, hash = ?value.value, "Replaying already known proposed value");

                    consensus.cast(ConsensusMsg::ReceivedProposedValue(
                        value,
                        ValueOrigin::Consensus,
                    ))?;
                }
                Ok(())
        })
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
        if let Some(value) = state.with_db_storage(|storage, db| {
            db.undecided_proposals
                .idx
                .proposer
                .may_load_value(storage, (*height, round.as_i64(), self.address))
        })? {
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

            let hash = hasher.finalize();
            let sig = self.private_key.sign_digest(hash);

            let parts = StoreParts::new(
                ProposalInit::new(height, round, self.address),
                txs,
                ProposalFin::new(hash, sig),
            );

            (parts, hash)
        };

        // Store parts
        let stream_id = state.stream_id();
        // PARTS.save(state, stream_id.to_bytes().to_vec(), &parts)?;

        // Stream parts to consensus
        self.stream_parts(stream_id, parts, true).await?;

        // Return the proposed value
        {
            let value = LocallyProposedValue::new(height, round, hash.into());
            reply_to.send(value)?;
        }

        Ok(())
    }

    async fn restream_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        round: Round,
        valid_round: Round,
        address: ctx!(Address),
        value_id: ctx!(Value::Id),
    ) -> ActorResult<()> {
        if let Some(mut parts) = state.with_memory_storage(|storage, db| {
            db.parts.idx.value_id.may_load_value(storage, value_id)
        })? {
            // recreate fin and init parts
            parts.init = ProposalInit::new_with_valid_round(height, round, address, valid_round);
            let sig = self.private_key.sign_digest(value_id.into_inner());
            parts.fin = ProposalFin::new(value_id.into_inner(), sig);

            self.stream_parts(state.stream_id(), parts, false).await?;
        }

        Ok(())
    }

    async fn received_proposal_part(
        &self,
        state: &mut State,
        from: PeerId,
        part: StreamMessage<ctx!(ProposalPart)>,
        reply_to: RpcReplyPort<ProposedValue<Context>>,
    ) -> ActorResult<()> {
        let Some(parts) = state.add_part(from, part) else {
            return Ok(());
        };

        Ok(())
    }
}
