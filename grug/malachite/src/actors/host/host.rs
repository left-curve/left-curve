use {
    crate::{
        ActorResult, HostConfig,
        app::{HostApp, HostAppRef},
        context::Context,
        ctx,
        host::state::State,
        mempool::{MempoolActorRef, MempoolMsg},
        types::{Block, DecidedBlock, PreBlock, ProposalFin, ProposalInit, ProposalParts},
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
    malachitebft_sync::{PeerId, RawDecidedValue},
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
    private_key: ctx!(SigningScheme::PrivateKey),
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
        app: Arc<App<DB, VM, PP, ID>>,
        mempool: MempoolActorRef,
        network: NetworkRef<Context>,
        validator_set: ctx!(ValidatorSet),
        private_key: ctx!(SigningScheme::PrivateKey),
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
            address: private_key.derive_address(),
            private_key,
            span,
        };

        let (actor_ref, _) = Actor::spawn(None, host, args).await.unwrap();
        actor_ref
    }

    pub async fn stream_parts(
        &self,
        stream_id: StreamId,
        parts: ProposalParts,
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
            HostMsg::ReceivedProposalPart {
                from,
                part,
                reply_to,
            } => {
                self.received_proposal_part(state, from, part, reply_to)
                    .await
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
        let height = state
            .with_db_storage(|storage, db| {
                db.decided_block
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
                db.decided_block
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

        state.started_round();

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

        let parts = block.as_parts(&self.private_key);

        let stream_id = state.stream_id();

        // Store parts
        {
            state.with_memory_storage_mut(|storage, memory| {
                memory
                    .parts
                    .save(storage, stream_id.to_bytes().to_vec(), &parts)
            })?;
        }

        // Store undecided proposal
        {
            let proposed_value = ProposedValue {
                height,
                round,
                valid_round: Round::Nil,
                proposer: self.address,
                value: <ctx!(Value)>::new(parts.fin.hash),
                validity: Validity::Valid,
            };

            state.with_db_storage_mut(|storage, db| {
                db.undecided_proposals.save(
                    storage,
                    (*height, round.as_i64(), parts.fin.hash),
                    &proposed_value,
                )
            })?;
        }

        // Store undecided block
        {
            state.with_db_storage_mut(|storage, db| {
                db.undecided_block
                    .save(storage, (*height, round.as_i64(), parts.fin.hash), &block)
            })?;
        }

        let value = LocallyProposedValue::new(height, round, <ctx!(Value)>::new(parts.fin.hash));

        // Stream parts to consensus
        self.stream_parts(stream_id, parts, true).await?;

        // Return the proposed value
        reply_to.send(value)?;

        Ok(())
    }

    #[tracing::instrument("received_proposal_part", skip_all)]
    async fn received_proposal_part(
        &self,
        state: &mut State,
        from: PeerId,
        part: StreamMessage<ctx!(ProposalPart)>,
        reply_to: RpcReplyPort<ProposedValue<Context>>,
    ) -> ActorResult<()> {
        let stream_id = part.stream_id.clone();

        let Some(parts) = state.buffer_part(from, part) else {
            return Ok(());
        };

        info!(height = %parts.init.height, round = %parts.init.round, "All parts received");

        // Run FinalizeBlock
        let block = parts.clone().into_pre_block();
        let app_hash = self.app.finalize_block(block.as_block_info(), &block.txs)?;
        let block = block.with_app_hash(app_hash);

        let block_hash = block.hash();

        let value = ProposedValue {
            height: parts.init.height,
            round: parts.init.round,
            valid_round: parts.init.valid_round,
            proposer: parts.init.proposer,
            value: <ctx!(Value)>::new(parts.fin.hash),
            validity: Validity::from_bool(parts.fin.hash == block_hash),
        };

        if value.validity == Validity::Valid {
            info!(block_hash = %block_hash, "Block hash matches");
            // Store undecided proposal
            state.with_db_storage_mut(|storage, db| {
                db.undecided_proposals.save(
                    storage,
                    (
                        *parts.init.height,
                        parts.init.round.as_i64(),
                        parts.fin.hash,
                    ),
                    &value,
                )
            })?;

            // Store undecided block
            state.with_db_storage_mut(|storage, db| {
                db.undecided_block.save(
                    storage,
                    (
                        *parts.init.height,
                        parts.init.round.as_i64(),
                        parts.fin.hash,
                    ),
                    &block,
                )
            })?;

            // Store parts
            state.with_memory_storage_mut(|storage, memory| {
                memory
                    .parts
                    .save(storage, stream_id.to_bytes().to_vec(), &parts)
            })?;
        } else {
            warn!(block_hash = %block_hash, proposal_block_hash = %parts.fin.hash, "Block hash mismatch");
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
        let Some(block) = state.with_db_storage(|storage, db| {
            db.undecided_block.may_load(
                storage,
                (
                    *certificate.height,
                    certificate.round.as_i64(),
                    certificate.value_id,
                ),
            )
        })?
        else {
            warn!(value_id = %certificate.value_id, "Proposed block not found");
            return Err(anyhow::anyhow!("Proposed block not found").into());
        };

        info!(value_id = %certificate.value_id, "Proposed block found");

        // Call commit
        self.app.commit()?;

        let txs = block.txs.clone();

        // Store decided block
        state.with_db_storage_mut(|storage, db| {
            db.decided_block
                .save(storage, *certificate.height, &DecidedBlock {
                    block,
                    certificate,
                })
        })?;

        // Notify the mempool to remove corresponding txs
        self.mempool.cast(MempoolMsg::Remove(txs))?;

        // TODO: Notify the Host of the decision.
        // Is this the equivalent to call commit into the App?

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

    async fn restream_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        round: Round,
        valid_round: Round,
        address: ctx!(Address),
        block_hash: ctx!(Value::Id),
    ) -> ActorResult<()> {
        if let Some(mut parts) = state.with_memory_storage(|storage, db| {
            db.parts.idx.value_id.may_load_value(storage, block_hash)
        })? {
            // recreate fin and init parts
            parts.init = ProposalInit::new_with_valid_round(
                height,
                address,
                round,
                parts.init.timestamp,
                valid_round,
            );
            // TODO: Should we re-sign the block hash? Or just restream?
            let sig = self.private_key.sign_digest(block_hash);
            parts.fin = ProposalFin::new(block_hash, sig);

            self.stream_parts(state.stream_id(), parts, false).await?;
        }

        Ok(())
    }

    async fn get_decided_value(
        &self,
        state: &mut State,
        height: ctx!(Height),
        reply_to: RpcReplyPort<Option<RawDecidedValue<Context>>>,
    ) -> ActorResult<()> {
        let Some(decided_block) =
            state.with_db_storage(|storage, db| db.decided_block.may_load(storage, *height))?
        else {
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
        let Ok(mut block): Result<Block, _> = value.deserialize_borsh() else {
            return Ok(());
        };

        let received_block_hash = block.hash();

        // Check validity
        let app_hash = self.app.finalize_block(block.as_block_info(), &block.txs)?;
        block.override_app_hash(app_hash);

        let validity = received_block_hash == block.hash();

        // Store undecided block
        state.with_db_storage_mut(|storage, db| {
            db.undecided_block.save(
                storage,
                (*height, round.as_i64(), received_block_hash),
                &block,
            )
        })?;

        let proposed_value = ProposedValue {
            height,
            round,
            valid_round: Round::Nil,
            proposer: validator_address,
            value: <ctx!(Value)>::new(received_block_hash),
            validity: Validity::from_bool(validity),
        };

        // Store undecided proposal
        state.with_db_storage_mut(|storage, db| {
            db.undecided_proposals.save(
                storage,
                (*height, round.as_i64(), received_block_hash),
                &proposed_value,
            )
        })?;

        reply_to.send(proposed_value)?;

        Ok(())
    }
}
