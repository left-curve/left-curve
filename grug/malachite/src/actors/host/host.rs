use {
    crate::{
        ActorResult, HostConfig, PreBlock, ProposalData,
        app::{HostApp, HostAppRef},
        context::Context,
        ctx,
        host::state::{DECIDED_BLOCK, State, UNDECIDED_BLOCK, UNDECIDED_PROPOSALS},
        mempool::{MempoolActorRef, MempoolMsg},
        types::{Block, DecidedBlock},
    },
    grug::{BorshDeExt, BorshSerExt, Timestamp},
    grug_app::{App, Db},
    malachitebft_app::{
        consensus::Role,
        streaming::{StreamContent, StreamId, StreamMessage},
        types::{LocallyProposedValue, ProposedValue},
    },
    malachitebft_core_types::{CommitCertificate, Height, Round, Validity, ValueOrigin},
    malachitebft_engine::{
        consensus::{ConsensusMsg, ConsensusRef},
        network::{NetworkMsg, NetworkRef},
    },
    malachitebft_network::Bytes,
    malachitebft_sync::RawDecidedValue,
    ractor::{Actor, RpcReplyPort, async_trait},
    std::{sync::Arc, time::Duration},
    tracing::{Span, error, info, warn},
};

pub type HostRef = malachitebft_engine::host::HostRef<Context>;
pub type HostMsg = malachitebft_engine::host::HostMsg<Context>;

pub struct Host {
    app: HostAppRef,
    mempool: MempoolActorRef,
    network: NetworkRef<Context>,
    validator_set: ctx!(ValidatorSet),
    address: ctx!(Address),
    span: Span,
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

    #[tracing::instrument("host", parent = &self.span, skip_all)]
    async fn handle(
        &self,
        _: HostRef,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> ActorResult<()> {
        if let Err(e) = self.handle_msg(message, state).await {
            error!("Error handling message: {:?}", e);
        }

        Ok(())
    }
}

//  ----------------------------- Generic methods ------------------------------

impl Host {
    pub async fn spawn<DB, VM, PP, ID>(
        address: ctx!(Address),
        app: Arc<App<DB, VM, PP, ID>>,
        mempool: MempoolActorRef,
        network: NetworkRef<Context>,
        validator_set: ctx!(ValidatorSet),
        span: Span,
        config: HostConfig,
    ) -> HostRef
    where
        DB: Db,
        App<DB, VM, PP, ID>: HostApp,
    {
        let args = State::new(app.db.consensus(), config);

        let host = Host {
            app,
            mempool,
            network,
            validator_set,
            address,
            span,
        };

        let (actor_ref, _) = Actor::spawn(None, host, args).await.unwrap();
        actor_ref
    }

    pub async fn stream_data(
        &self,
        stream_id: StreamId,
        data: ProposalData,
        with_fin: bool,
    ) -> ActorResult<()> {
        let msg = StreamMessage::new(stream_id.clone(), 0, StreamContent::Data(data));
        self.network.cast(NetworkMsg::PublishProposalPart(msg))?;

        if with_fin {
            let msg = StreamMessage::new(stream_id.clone(), 1, StreamContent::Fin);
            self.network.cast(NetworkMsg::PublishProposalPart(msg))?;
        }

        Ok(())
    }
}

// ------------------------------- Msgs handlers -------------------------------

impl Host {
    async fn handle_msg(&self, message: HostMsg, state: &mut State) -> ActorResult<()> {
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
            HostMsg::ReceivedProposalPart { part, reply_to, .. } => {
                self.received_proposal_part(state, part, reply_to).await
            },
            HostMsg::GetValidatorSet { reply_to, .. } => {
                reply_to.send(Some(self.validator_set.clone()))?;
                Ok(())
            },
            HostMsg::Decided {
                certificate,
                consensus,
                ..
            } => self.decided(state, certificate, consensus).await,
            HostMsg::GetDecidedValue { height, reply_to } => {
                self.get_decided_value(state, height, reply_to).await
            },
            HostMsg::ProcessSyncedValue {
                height,
                round,
                validator_address,
                value_bytes,
                reply_to,
            } => {
                self.process_synced_value(
                    state,
                    height,
                    round,
                    validator_address,
                    value_bytes,
                    reply_to,
                )?;

                Ok(())
            },
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

    fn consensus_ready(
        &self,
        state: &mut State,
        consensus: ConsensusRef<Context>,
    ) -> ActorResult<()> {
        let height = DECIDED_BLOCK
            .keys(state, None, None, grug::Order::Descending)
            .next()
            .transpose()?
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
        let height = DECIDED_BLOCK
            .keys(state, None, None, grug::Order::Ascending)
            .next()
            .transpose()?
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
        state.started_round(height, round, proposer, role);

        // If we have already built or seen one or more values for this height and round,
        // feed them back to consensus. This may happen when we are restarting after a crash.
        let consensus = state.consensus()?;

        for value in UNDECIDED_PROPOSALS
            .prefix(*height)
            .append(round.as_i64())
            .values(state, None, None, grug::Order::Ascending)
        {
            let value = value?;
            info!(%height, %round, hash = ?value.value, "Replaying already known proposed value");

            consensus.cast(ConsensusMsg::ReceivedProposedValue(
                value,
                ValueOrigin::Consensus,
            ))?;
        }
        Ok(())
    }

    async fn restream_value(
        &self,
        state: &mut State,
        _height: ctx!(Height),
        _round: Round,
        valid_round: Round,
        _address: ctx!(Address),
        block_hash: ctx!(Value::Id),
    ) -> ActorResult<()> {
        if let Some(data) = UNDECIDED_BLOCK.may_load(state, block_hash)? {
            // TODO: Should we assert height, round and proposer?

            // recreate fin and init parts
            let data = ProposalData {
                block: data,
                valid_round,
            };

            self.stream_data(state.stream_id(), data, false).await?;
        }

        Ok(())
    }

    /// Equivalent of prepare_proposal
    #[tracing::instrument("get_value", skip_all, fields(height = %height, round = %round))]
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
        if let Some(value) = UNDECIDED_PROPOSALS
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
        let block = {
            // Take txs from mempool
            info!("Taking txs from mempool");
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

            // Call prepare_proposal
            info!("Calling prepare_proposal");
            let txs = self.app.prepare_proposal(txs);

            let pre_block = PreBlock::new(
                height,
                self.address,
                round,
                Timestamp::from_nanos(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                ),
                txs,
            );

            info!("Calling finalize_block");
            let app_hash = self
                .app
                .finalize_block(pre_block.as_block_info(), &pre_block.txs)?;

            pre_block.with_app_hash(app_hash)
        };

        let proposal_data = block.as_proposal_data();

        let stream_id = state.stream_id();

        // Store undecided proposal
        let proposed_value = ProposedValue {
            height,
            round,
            valid_round: Round::Nil,
            proposer: self.address,
            value: <ctx!(Value)>::new(proposal_data.block.block_hash()),
            validity: Validity::Valid,
        };

        UNDECIDED_PROPOSALS.save(
            state,
            (*height, round.as_i64(), proposal_data.block.block_hash()),
            &proposed_value,
        )?;

        // Store undecided block
        UNDECIDED_BLOCK.save(state, proposal_data.block.block_hash(), &block)?;

        let value = LocallyProposedValue::new(
            height,
            round,
            <ctx!(Value)>::new(proposal_data.block.block_hash()),
        );

        // Stream parts to consensus
        self.stream_data(stream_id, proposal_data, true).await?;

        // Return the proposed value
        reply_to.send(value)?;

        Ok(())
    }

    #[tracing::instrument("received_proposal_part", skip_all)]
    async fn received_proposal_part(
        &self,
        state: &mut State,
        part: StreamMessage<ctx!(ProposalPart)>,
        reply_to: RpcReplyPort<ProposedValue<Context>>,
    ) -> ActorResult<()> {
        // Since we are using a single part proposal, we only need data
        let Some(ProposalData { block, valid_round }) = part.content.into_data() else {
            return Ok(());
        };

        info!(height = %block.height, round = %block.round, "All parts received");

        // Run FinalizeBlock
        let app_hash = self.app.finalize_block(block.as_block_info(), &block.txs)?;
        let block_hash = block.calculate_block_hash(app_hash);

        let value = ProposedValue {
            height: block.height,
            round: block.round,
            valid_round,
            proposer: block.proposer,
            // TODO: Is this correct? If block hashes are different, which one we should use?
            value: <ctx!(Value)>::new(block.block_hash()),
            validity: Validity::from_bool(block.block_hash() == block_hash),
        };

        if value.validity == Validity::Valid {
            info!(block_hash = %block_hash, "Block hash matches");
            // Store undecided proposal
            UNDECIDED_PROPOSALS.save(
                state,
                (*block.height, block.round.as_i64(), block.block_hash()),
                &value,
            )?;

            // Store undecided block
            UNDECIDED_BLOCK.save(state, block.block_hash(), &block)?;
        } else {
            warn!(block_hash = %block_hash, proposal_block_hash = %block.block_hash(), "Block hash mismatch");
        }

        // TODO: Should we resign the block hash?

        // TODO: If block_hash is different from the proposed value, what we should do?
        // store the undecided block with the new block_hash?

        // Send to consensus
        reply_to.send(value)?;

        Ok(())
    }

    /// Equivalent of commit
    #[tracing::instrument("decided", skip_all, fields(height = %certificate.height, round = %certificate.round))]
    async fn decided(
        &self,
        state: &mut State,
        certificate: CommitCertificate<Context>,
        consensus: ConsensusRef<Context>,
    ) -> ActorResult<()> {
        let Some(block) = UNDECIDED_BLOCK.may_load(state, certificate.value_id)? else {
            warn!(value_id = %certificate.value_id, "Proposed block not found");
            return Err(anyhow::anyhow!("Proposed block not found").into());
        };

        info!(value_id = %certificate.value_id, "Proposed block found");

        // Call commit
        self.app.commit()?;

        let txs = block.txs.clone();

        // Store decided block
        DECIDED_BLOCK.save(state, *certificate.height, &DecidedBlock {
            block,
            certificate,
        })?;

        // Notify the mempool to remove corresponding txs
        self.mempool.cast(MempoolMsg::Remove(txs))?;

        // TODO: Prune the undecided blocks and proposals

        // Start the next height
        let sleep = state.calculate_block_sleep();
        let validator_set = self.validator_set.clone();
        let next_height = state.height.increment();
        info!(diff = ?sleep, "sleeping until next round");

        tokio::spawn(async move {
            tokio::time::sleep(sleep).await;
            if let Err(e) = consensus.cast(ConsensusMsg::StartHeight(next_height, validator_set)) {
                error!("Error starting height: {:?}", e);
            }
        });

        Ok(())
    }

    async fn get_decided_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        reply_to: RpcReplyPort<Option<RawDecidedValue<Context>>>,
    ) -> ActorResult<()> {
        let Some(decided_block) = DECIDED_BLOCK.may_load(state, *height)? else {
            reply_to.send(None)?;
            return Ok(());
        };

        let decided_value = RawDecidedValue::new(
            decided_block.block.to_borsh_vec()?.into(),
            decided_block.certificate.clone(),
        );

        reply_to.send(Some(decided_value))?;

        Ok(())
    }

    fn process_synced_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        round: Round,
        validator_address: ctx!(Address),
        value: Bytes,
        reply_to: RpcReplyPort<ProposedValue<Context>>,
    ) -> ActorResult<()> {
        let Ok(block): Result<Block, _> = value.deserialize_borsh() else {
            return Ok(());
        };

        // Check validity
        let app_hash = self.app.finalize_block(block.as_block_info(), &block.txs)?;
        let block_hash = block.calculate_block_hash(app_hash);

        let proposed_value = ProposedValue {
            height,
            round,
            valid_round: Round::Nil,
            proposer: validator_address,
            value: <ctx!(Value)>::new(block.block_hash()),
            validity: Validity::from_bool(block_hash == block.block_hash()),
        };

        if proposed_value.validity == Validity::Valid {
            // Store undecided block
            UNDECIDED_BLOCK.save(state, block_hash, &block)?;

            // Store undecided proposal

            UNDECIDED_PROPOSALS.save(
                state,
                (*height, round.as_i64(), block_hash),
                &proposed_value,
            )?;
        }

        reply_to.send(proposed_value)?;

        Ok(())
    }
}
