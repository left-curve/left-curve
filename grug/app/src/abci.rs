use {
    crate::{App, AppError, AppResult, Db, Indexer, ProposalPreparer, Vm},
    grug_types::{
        BlockInfo, CheckTxOutcome, Duration, GENESIS_BLOCK_HASH, GenericResult, Hash256, Inner,
        JsonSerExt, TxOutcome,
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
        AppHash, Hash, Time,
        abci::{self, Code, request, response, types::ExecTxResult},
        block::Height,
        merkle::proof::{ProofOp, ProofOps},
        v0_38::abci::{Request, Response},
    },
    tower::Service,
    tower_abci::BoxError,
};

impl<DB, VM, PP, ID> Service<Request> for App<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    ID: Indexer,
    PP: ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    type Response = Response;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let res = self.tower_call(req);
        Box::pin(async move { res.map_err(|err| Box::new(err) as BoxError) })
    }
}

impl<DB, VM, PP, ID> App<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    ID: Indexer,
    PP: ProposalPreparer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn tower_call(&self, req: Request) -> AppResult<Response> {
        match req {
            // -------------------- block execution methods --------------------
            Request::InitChain(req) => {
                let res = self.tower_init_chain(req)?;
                Ok(Response::InitChain(res))
            },
            Request::PrepareProposal(req) => {
                let res = self.tower_prepare_proposal(req)?;
                Ok(Response::PrepareProposal(res))
            },
            Request::ProcessProposal(_) => {
                // Always accept.
                let res = response::ProcessProposal::Accept;
                Ok(Response::ProcessProposal(res))
            },
            Request::ExtendVote(_) => {
                // Vote extension isn't supported yet. Do nothing.
                let res = response::ExtendVote {
                    vote_extension: Bytes::default(),
                };
                Ok(Response::ExtendVote(res))
            },
            Request::VerifyVoteExtension(_) => {
                // Always accept.
                let res = response::VerifyVoteExtension::Accept;
                Ok(Response::VerifyVoteExtension(res))
            },
            Request::FinalizeBlock(req) => {
                let res = self.tower_finalize_block(req)?;
                Ok(Response::FinalizeBlock(res))
            },
            Request::Commit => {
                let res = self.tower_commit()?;
                Ok(Response::Commit(res))
            },

            // ------------------------ mempool methods ------------------------
            Request::CheckTx(req) => {
                let res = self.tower_check_tx(req)?;
                Ok(Response::CheckTx(res))
            },

            // ------------------------- query methods -------------------------
            Request::Info(..) => {
                let res = self.tower_info()?;
                Ok(Response::Info(res))
            },
            Request::Query(req) => {
                let res = self.tower_query(req)?;
                Ok(Response::Query(res))
            },

            // ---------------------- state sync methods -----------------------
            Request::ListSnapshots => Ok(Response::ListSnapshots(Default::default())),
            Request::OfferSnapshot(_) => Ok(Response::OfferSnapshot(Default::default())),
            Request::LoadSnapshotChunk(_) => Ok(Response::LoadSnapshotChunk(Default::default())),
            Request::ApplySnapshotChunk(_) => Ok(Response::ApplySnapshotChunk(Default::default())),

            // ------------------------- other methods -------------------------
            Request::Echo(req) => {
                let res = response::Echo {
                    message: req.message,
                };
                Ok(Response::Echo(res))
            },
            Request::Flush => Ok(Response::Flush),
        }
    }

    fn tower_check_tx(&self, req: request::CheckTx) -> AppResult<response::CheckTx> {
        // Note: We don't have separate logics for `CheckTyType::New` vs `Recheck`.
        let res = match self.do_check_tx_raw(&req.tx) {
            Ok(CheckTxOutcome {
                result: GenericResult::Ok(_),
                gas_limit,
                events,
                gas_used,
            }) => response::CheckTx {
                code: Code::Ok,
                data: events.to_json_vec()?.into(),
                gas_wanted: gas_limit as i64,
                gas_used: gas_used as i64,
                ..Default::default()
            },
            Ok(CheckTxOutcome {
                result: GenericResult::Err(err),
                gas_limit,
                events,
                gas_used,
            }) => response::CheckTx {
                code: into_tm_code_error(1),
                codespace: "check_tx".into(),
                data: events.to_json_vec()?.into(),
                gas_wanted: gas_limit as i64,
                gas_used: gas_used as i64,
                log: err,
                ..Default::default()
            },
            Err(err) => response::CheckTx {
                code: into_tm_code_error(1),
                codespace: "check_tx".into(),
                log: err.to_string(),
                ..Default::default()
            },
        };

        Ok(res)
    }

    fn tower_commit(&self) -> AppResult<response::Commit> {
        match self.do_commit() {
            Ok(()) => Ok(response::Commit {
                // This field is ignored since CometBFT 0.38.
                // TODO: Can we omit this?????
                data: Default::default(),
                // TODO: what this means??
                retain_height: Height::default(),
            }),
            Err(err) => panic!("failed to commit: {err}"),
        }
    }

    fn tower_finalize_block(
        &self,
        req: request::FinalizeBlock,
    ) -> AppResult<response::FinalizeBlock> {
        let block = from_tm_block(req.height.value(), req.time, Some(req.hash));

        match self.do_finalize_block_raw(block, &req.txs) {
            Ok(outcome) => {
                let tx_results = outcome
                    .tx_outcomes
                    .into_iter()
                    .map(into_tm_tx_result)
                    .collect::<AppResult<_>>()?;

                let cron_events = outcome
                    .cron_outcomes
                    .into_iter()
                    .enumerate()
                    .map(|(id, cron)| {
                        Ok(abci::Event {
                            kind: format!("cron-{}", id),
                            attributes: vec![abci::EventAttribute::V037(
                                abci::v0_37::EventAttribute {
                                    key: format!("cron-{}", id),
                                    value: cron.to_json_string()?,
                                    index: false,
                                },
                            )],
                        })
                    })
                    .collect::<AppResult<_>>()?;

                Ok(response::FinalizeBlock {
                    app_hash: into_tm_app_hash(outcome.app_hash),
                    // `events` field is used for cron events.
                    events: cron_events,
                    tx_results,
                    // We haven't implemented any mechanism to alter the
                    // validator set or consensus params yet.
                    validator_updates: vec![],
                    consensus_param_updates: None,
                })
            },
            Err(err) => panic!("failed to finalize block: {err}"),
        }
    }

    fn tower_info(&self) -> AppResult<response::Info> {
        let (last_block_height, last_block_version) = self.do_info()?;

        Ok(response::Info {
            data: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            app_version: 1,
            last_block_height: last_block_height
                .try_into()
                .expect("block height exceeds i64"),
            last_block_app_hash: into_tm_app_hash(last_block_version),
        })
    }

    fn tower_init_chain(&self, req: request::InitChain) -> AppResult<response::InitChain> {
        let block = from_tm_block(0, req.time, None);

        match self.do_init_chain_raw(req.chain_id, block, &req.app_state_bytes) {
            Ok(app_hash) => Ok(response::InitChain {
                consensus_params: Some(req.consensus_params),
                validators: req.validators,
                app_hash: into_tm_app_hash(app_hash),
            }),
            Err(err) => panic!("failed to init chain: {err}"),
        }
    }

    fn tower_prepare_proposal(
        &self,
        req: request::PrepareProposal,
    ) -> AppResult<response::PrepareProposal> {
        let max_tx_bytes = req.max_tx_bytes.try_into().unwrap_or(0);
        let txs = self.do_prepare_proposal(req.txs.clone(), max_tx_bytes);

        Ok(response::PrepareProposal { txs })
    }

    fn tower_query(&self, req: request::Query) -> AppResult<response::Query> {
        let res = match req.path.as_str() {
            "/app" => match self.do_query_app_raw(&req.data, req.height.value(), req.prove) {
                Ok(res) => response::Query {
                    code: Code::Ok,
                    value: res.into(),
                    ..Default::default()
                },
                Err(err) => response::Query {
                    code: into_tm_code_error(1),
                    codespace: "app".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            "/simulate" => match self.do_simulate_raw(&req.data, req.height.value(), req.prove) {
                Ok(outcome) => response::Query {
                    code: Code::Ok,
                    value: outcome.into(),
                    ..Default::default()
                },
                Err(err) => response::Query {
                    code: into_tm_code_error(1),
                    codespace: "simulate".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            "/store" => match self.do_query_store(&req.data, req.height.value(), req.prove) {
                Ok((value, proof)) => {
                    let proof = proof.map(|proof| ProofOps {
                        ops: vec![ProofOp {
                            field_type: type_name::<DB::Proof>().into(),
                            key: req.data.into(),
                            data: proof,
                        }],
                    });
                    response::Query {
                        code: Code::Ok,
                        value: value.unwrap_or_default().into(),
                        height: req.height,
                        proof,
                        ..Default::default()
                    }
                },
                Err(err) => response::Query {
                    code: into_tm_code_error(1),
                    codespace: "store".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            unknown => response::Query {
                code: into_tm_code_error(1),
                codespace: "app".into(),
                log: format!("unknown path `{unknown}`; must be `/app`, `/simulate`, or `/store`"),
                ..Default::default()
            },
        };

        Ok(res)
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

fn into_tm_tx_result(outcome: TxOutcome) -> AppResult<ExecTxResult> {
    let (code, codespace) = if outcome.result.is_ok() {
        (Code::Ok, "")
    } else {
        (into_tm_code_error(1), "tx")
    };

    Ok(ExecTxResult {
        code,
        data: outcome.events.to_json_vec()?.into(),
        codespace: codespace.to_string(),
        log: outcome.result.to_json_string()?,
        gas_wanted: outcome.gas_limit as i64,
        gas_used: outcome.gas_used as i64,
        ..Default::default()
    })
}

fn into_tm_app_hash(hash: Hash256) -> AppHash {
    hash.into_inner().to_vec().try_into().unwrap()
}

/// Be sure to pass a non-zero error code.
fn into_tm_code_error(code: u32) -> Code {
    Code::Err(NonZeroU32::new(code).unwrap_or_else(|| {
        panic!("expected non-zero error code, got {code}");
    }))
}
