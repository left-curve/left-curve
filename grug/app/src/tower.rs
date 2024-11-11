use {
    crate::{App, AppError, Db, NaiveProposalPreparer, NaiveQuerier, ProposalPreparer, Vm},
    grug_math::Inner,
    grug_types::{
        Attribute, BlockInfo, Duration, Event, GenericResult, Hash256, Outcome, QuerierWrapper,
        TxOutcome, GENESIS_BLOCK_HASH,
    },
    prost::bytes::Bytes,
    std::{
        any::type_name,
        future::Future,
        num::NonZeroU32,
        pin::Pin,
        task::{Context, Poll},
    },
    tendermint::{
        abci::{
            response::{
                CheckTx, Commit, ExtendVote, FinalizeBlock, Info, InitChain, PrepareProposal,
                Query, VerifyVoteExtension,
            },
            types::ExecTxResult,
            v0_34::EventAttribute,
            Code, Event as TmEvent, EventAttribute as TmAttribute,
        },
        block::Height,
        merkle::proof::{ProofOp, ProofOps},
        v0_38::abci::{Request, Response},
        Hash, Time,
    },
    tower::Service,
    tracing::error,
};

impl<DB, VM, PP> Service<Request> for App<DB, VM, PP>
where
    DB: Db,
    VM: Vm + Clone,
    PP: ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Response = Response;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let res = match req {
            Request::Info(..) => {
                let (last_block_height, last_block_version) = self.do_info().unwrap();
                Response::Info(Info {
                    data: env!("CARGO_PKG_NAME").into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    app_version: 1,
                    last_block_height: last_block_height.try_into().unwrap(),
                    last_block_app_hash: last_block_version
                        .into_inner()
                        .to_vec()
                        .try_into()
                        .unwrap(),
                })
            },

            Request::InitChain(req) => {
                let block = from_tm_block(0, req.time, None);

                match self.do_init_chain_raw(req.chain_id, block, &req.app_state_bytes) {
                    Ok(app_hash) => Response::InitChain(InitChain {
                        consensus_params: Some(req.consensus_params),
                        validators: req.validators,
                        app_hash: app_hash.into_inner().to_vec().try_into().unwrap(),
                    }),
                    Err(err) => panic!("failed to init chain: {err}"),
                }
            },

            Request::PrepareProposal(req) => {
                let max_tx_bytes = req.max_tx_bytes.try_into().unwrap_or(0);

                let txs = self
                    .do_prepare_proposal(req.txs.clone(), max_tx_bytes)
                    .unwrap_or_else(|err| {
                        // For the sake of liveness, in case proposal preparation fails,
                        // we fall back to the naive strategy instead of panicking.
                        #[cfg(feature = "tracing")]
                        error!(
                            err = err.to_string(),
                            "Failed to prepare proposal! Falling back to naive preparer."
                        );

                        NaiveProposalPreparer
                            .prepare_proposal(
                                QuerierWrapper::new(&NaiveQuerier),
                                req.txs,
                                max_tx_bytes,
                            )
                            .unwrap()
                    });

                Response::PrepareProposal(PrepareProposal { txs })
            },

            Request::FinalizeBlock(req) => {
                let block = from_tm_block(req.height.value(), req.time, Some(req.hash));

                match self.do_finalize_block_raw(block, &req.txs) {
                    Ok(outcome) => {
                        // In Cosmos SDK, this refers to the Begin/EndBlocker events.
                        // For us, this is the cronjob events.
                        // Note that failed cronjobs are ignored (not included in `ResponseFinalizeBlock`).
                        let events = outcome
                            .cron_outcomes
                            .into_iter()
                            .filter_map(|outcome| outcome.result.ok().map(into_tm_events))
                            .flatten()
                            .collect();

                        let tx_results = outcome
                            .tx_outcomes
                            .into_iter()
                            .map(into_tm_tx_result)
                            .collect();

                        Response::FinalizeBlock(FinalizeBlock {
                            app_hash: outcome.app_hash.into_inner().to_vec().try_into().unwrap(),
                            events,
                            tx_results,
                            // We haven't implemented any mechanism to alter the
                            // validator set or consensus params yet.
                            validator_updates: vec![],
                            consensus_param_updates: None,
                        })
                    },
                    Err(err) => panic!("failed to finalize block: {err}"),
                }
            },

            Request::Commit => match self.do_commit() {
                Ok(()) => Response::Commit(Commit {
                    // This field is ignored since CometBFT 0.38.
                    // TODO: Can we omit this?????
                    data: Default::default(),
                    // TODO: what this means??
                    retain_height: Height::default(),
                }),
                Err(err) => panic!("failed to commit: {err}"),
            },

            Request::CheckTx(req) => {
                let res = match self.do_check_tx_raw(&req.tx) {
                    Ok(Outcome {
                        result: GenericResult::Ok(events),
                        gas_limit,
                        ..
                    }) => CheckTx {
                        code: Code::Ok,
                        events: into_tm_events(events),
                        gas_wanted: gas_limit.unwrap() as i64,
                        // Note: Return `Outcome::gas_limited` instead of `gas_used here.
                        // This is because in `CheckTx` we don't run the entire tx, just
                        // the authentication part. As such, the gas consumption is
                        // underestimated. Instead, the tx gas limit represents the max
                        // amount of gas this tx can possibly consume.
                        gas_used: gas_limit.unwrap() as i64,
                        ..Default::default()
                    },
                    Ok(Outcome {
                        result: GenericResult::Err(err),
                        gas_limit,
                        ..
                    }) => CheckTx {
                        code: Code::Err(unsafe { NonZeroU32::new_unchecked(1) }),
                        codespace: "tx".into(),
                        log: err,
                        gas_wanted: gas_limit.unwrap() as i64,
                        gas_used: gas_limit.unwrap() as i64,
                        ..Default::default()
                    },
                    Err(err) => CheckTx {
                        code: Code::Err(unsafe { NonZeroU32::new_unchecked(1) }),
                        codespace: "simulate".into(),
                        log: err.to_string(),
                        ..Default::default()
                    },
                };

                Response::CheckTx(res)
            },

            Request::Query(req) => {
                let res = match req.path.as_str() {
                    "/app" => {
                        match self.do_query_app_raw(&req.data, req.height.value(), req.prove) {
                            Ok(res) => Query {
                                code: Code::Ok,
                                value: res.into(),
                                ..Default::default()
                            },
                            Err(err) => Query {
                                code: Code::Ok,
                                codespace: "app".into(),
                                log: err.to_string(),
                                ..Default::default()
                            },
                        }
                    },
                    "/simulate" => {
                        match self.do_simulate_raw(&req.data, req.height.value(), req.prove) {
                            Ok(outcome) => Query {
                                code: Code::Ok,
                                value: outcome.into(),
                                ..Default::default()
                            },
                            Err(err) => Query {
                                code: code_error(1),
                                codespace: "simulate".into(),
                                log: err.to_string(),
                                ..Default::default()
                            },
                        }
                    },
                    "/store" => {
                        match self.do_query_store(&req.data, req.height.value(), req.prove) {
                            Ok((value, proof)) => {
                                let proof = proof.map(|proof| ProofOps {
                                    ops: vec![ProofOp {
                                        field_type: type_name::<DB::Proof>().into(),
                                        key: req.data.into(),
                                        data: proof,
                                    }],
                                });
                                Query {
                                    code: Code::Ok,
                                    value: value.unwrap_or_default().into(),
                                    height: req.height,
                                    proof,
                                    ..Default::default()
                                }
                            },
                            Err(err) => Query {
                                code: code_error(1),
                                codespace: "store".into(),
                                log: err.to_string(),
                                ..Default::default()
                            },
                        }
                    },
                    unknown => Query {
                        code: code_error(1),
                        codespace: "app".into(),
                        log: format!(
                            "unknown path `{unknown}`; must be `/app`, `/simulate`, or `/store`"
                        ),
                        ..Default::default()
                    },
                };

                Response::Query(res)
            },

            // Unhandled requests
            Request::Flush => Response::Flush,
            Request::Echo(_) => Response::Echo(Default::default()),
            Request::ListSnapshots => Response::ListSnapshots(Default::default()),
            Request::OfferSnapshot(_) => Response::OfferSnapshot(Default::default()),
            Request::LoadSnapshotChunk(_) => Response::LoadSnapshotChunk(Default::default()),
            Request::ApplySnapshotChunk(_) => Response::ApplySnapshotChunk(Default::default()),
            Request::ProcessProposal(_) => Response::ProcessProposal(Default::default()),
            Request::ExtendVote(_) => Response::ExtendVote(ExtendVote {
                vote_extension: Bytes::default(),
            }),
            Request::VerifyVoteExtension(_) => {
                Response::VerifyVoteExtension(VerifyVoteExtension::Accept)
            },
        };

        Box::pin(async { Ok(res) })
    }
}

fn from_tm_block(height: u64, time: Time, hash: Option<Hash>) -> BlockInfo {
    BlockInfo {
        height,
        timestamp: Duration::from_nanos(time.unix_timestamp_nanos() as u128),
        hash: hash.map(from_tm_hash).unwrap_or(GENESIS_BLOCK_HASH),
    }
}

fn from_tm_hash(bytes: Hash) -> Hash256 {
    match bytes {
        Hash::Sha256(hash) => Hash256::from_inner(hash),
        Hash::None => panic!("unexpected empty hash"),
    }
}

fn into_tm_tx_result(outcome: TxOutcome) -> ExecTxResult {
    match outcome.result {
        GenericResult::Ok(_) => ExecTxResult {
            code: Code::Ok,
            gas_wanted: outcome.gas_limit as i64,
            gas_used: outcome.gas_used as i64,
            events: into_tm_events(outcome.events),
            ..Default::default()
        },
        GenericResult::Err(err) => ExecTxResult {
            code: Code::Err(unsafe { NonZeroU32::new_unchecked(1) }),
            codespace: "tx".to_string(),
            log: err,
            gas_wanted: outcome.gas_limit as i64,
            gas_used: outcome.gas_used as i64,
            events: into_tm_events(outcome.events),
            ..Default::default()
        },
    }
}
fn into_tm_events<I>(events: I) -> Vec<TmEvent>
where
    I: IntoIterator<Item = Event>,
{
    events.into_iter().map(into_tm_event).collect()
}

fn into_tm_event(event: Event) -> TmEvent {
    TmEvent {
        kind: event.r#type,
        attributes: into_tm_attributes(event.attributes),
    }
}

fn into_tm_attributes<I>(attrs: I) -> Vec<TmAttribute>
where
    I: IntoIterator<Item = Attribute>,
{
    attrs.into_iter().map(into_tm_attribute).collect()
}

fn into_tm_attribute(attr: Attribute) -> TmAttribute {
    // V037 is not exported from the `tendermint` crate.
    // IDK how to import it.
    TmAttribute::V034(EventAttribute {
        key: attr.key.as_bytes().to_vec(),
        value: attr.value.as_bytes().to_vec(),
        index: true,
    })
}

fn code_error(index: u32) -> Code {
    Code::Err(unsafe { NonZeroU32::new_unchecked(index) })
}
