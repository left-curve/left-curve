use {
    crate::{App, AppError, Db, NaiveProposalPreparer, NaiveQuerier, ProposalPreparer, Vm},
    grug_math::Inner,
    grug_types::{
        Attribute, BlockInfo, Duration, Event, GenericResult, Hash256, Outcome, QuerierWrapper,
        Timestamp, TxOutcome, GENESIS_BLOCK_HASH,
    },
    indexer_core::IndexerTrait as IndexerAppTrait,
    prost::bytes::Bytes,
    std::{any::type_name, net::ToSocketAddrs},
    tendermint_abci::{Application, Error as ABCIError, ServerBuilder},
    tendermint_proto::{
        abci::{
            Event as TmEvent, EventAttribute as TmAttribute, ExecTxResult, RequestCheckTx,
            RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestPrepareProposal,
            RequestQuery, ResponseCheckTx, ResponseCommit, ResponseFinalizeBlock, ResponseInfo,
            ResponseInitChain, ResponsePrepareProposal, ResponseQuery,
        },
        crypto::{ProofOp, ProofOps},
        google::protobuf::Timestamp as TmTimestamp,
    },
    tracing::error,
};

impl<DB, VM, INDEXER, PP> App<DB, VM, INDEXER, PP>
where
    DB: Db + Clone + Send + 'static,
    VM: Vm + Clone + Send + 'static,
    PP: ProposalPreparer + Clone + Send + 'static,
    INDEXER: IndexerAppTrait + Clone + Send + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    pub fn start_abci_server<A>(self, read_buf_size: usize, addr: A) -> Result<(), ABCIError>
    where
        A: ToSocketAddrs,
    {
        ServerBuilder::new(read_buf_size).bind(addr, self)?.listen()
    }
}

impl<DB, VM, INDEXER, PP> Application for App<DB, VM, INDEXER, PP>
where
    DB: Db + Clone + Send + 'static,
    VM: Vm + Clone + Send + 'static,
    PP: ProposalPreparer + Clone + Send + 'static,
    INDEXER: IndexerAppTrait + Clone + Send + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error>,
{
    fn info(&self, _req: RequestInfo) -> ResponseInfo {
        match self.do_info() {
            Ok((last_block_height, last_block_version)) => ResponseInfo {
                data: env!("CARGO_PKG_NAME").into(),
                version: env!("CARGO_PKG_VERSION").into(),
                app_version: 1,
                last_block_app_hash: last_block_version.into_inner().to_vec().into(),
                last_block_height: last_block_height as i64,
            },
            Err(err) => panic!("failed to get info: {err}"),
        }
    }

    fn init_chain(&self, req: RequestInitChain) -> ResponseInitChain {
        // Always use zero as the genesis height.
        // Ignore the block height from the ABCI request.
        //
        // It's mandatory that we genesis from block zero, such that block
        // height matches the DB version.
        //
        // It doesn't seem like setting the `initial_height` in CometBFT's
        // genesis file does anything. Setting it to zero, Comet still attmpts
        // to start at block 1.
        let block = from_tm_block(0, req.time, None);

        match self.do_init_chain_raw(req.chain_id, block, &req.app_state_bytes) {
            Ok(app_hash) => ResponseInitChain {
                consensus_params: req.consensus_params,
                validators: req.validators,
                app_hash: app_hash.into_inner().to_vec().into(),
            },
            Err(err) => panic!("failed to init chain: {err}"),
        }
    }

    fn prepare_proposal(&self, req: RequestPrepareProposal) -> ResponsePrepareProposal {
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
                    .prepare_proposal(QuerierWrapper::new(&NaiveQuerier), req.txs, max_tx_bytes)
                    .unwrap()
            });

        ResponsePrepareProposal { txs }
    }

    fn finalize_block(&self, req: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let block = from_tm_block(req.height, req.time, Some(req.hash));

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

                ResponseFinalizeBlock {
                    app_hash: outcome.app_hash.into_inner().to_vec().into(),
                    events,
                    tx_results,
                    // We haven't implemented any mechanism to alter the
                    // validator set or consensus params yet.
                    validator_updates: vec![],
                    consensus_param_updates: None,
                }
            },
            Err(err) => panic!("failed to finalize block: {err}"),
        }
    }

    fn commit(&self) -> ResponseCommit {
        match self.do_commit() {
            Ok(()) => {
                ResponseCommit {
                    retain_height: 0, // TODO: what this means??
                }
            },
            Err(err) => panic!("failed to commit: {err}"),
        }
    }

    fn check_tx(&self, req: RequestCheckTx) -> ResponseCheckTx {
        // Note: We don't have separate logics for `CheckTyType::New` vs `Recheck`.
        match self.do_check_tx_raw(&req.tx) {
            Ok(Outcome {
                result: GenericResult::Ok(events),
                gas_limit,
                ..
            }) => ResponseCheckTx {
                code: 0,
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
            }) => ResponseCheckTx {
                code: 1,
                codespace: "tx".into(),
                log: err,
                gas_wanted: gas_limit.unwrap() as i64,
                gas_used: gas_limit.unwrap() as i64,
                ..Default::default()
            },
            Err(err) => ResponseCheckTx {
                code: 1,
                codespace: "simulate".into(),
                log: err.to_string(),
                ..Default::default()
            },
        }
    }

    fn query(&self, req: RequestQuery) -> ResponseQuery {
        match req.path.as_str() {
            "/app" => match self.do_query_app_raw(&req.data, req.height as u64, req.prove) {
                Ok(res) => ResponseQuery {
                    code: 0,
                    value: res.into(),
                    ..Default::default()
                },
                Err(err) => ResponseQuery {
                    code: 1,
                    codespace: "app".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            "/simulate" => match self.do_simulate_raw(&req.data, req.height as u64, req.prove) {
                Ok(outcome) => ResponseQuery {
                    code: 0,
                    value: outcome.into(),
                    ..Default::default()
                },
                Err(err) => ResponseQuery {
                    code: 1,
                    codespace: "simulate".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            "/store" => match self.do_query_store(&req.data, req.height as u64, req.prove) {
                Ok((value, proof)) => {
                    let proof_ops = proof.map(|proof| ProofOps {
                        ops: vec![ProofOp {
                            r#type: type_name::<DB::Proof>().into(),
                            key: req.data.into(),
                            data: proof,
                        }],
                    });
                    ResponseQuery {
                        code: 0,
                        value: value.unwrap_or_default().into(),
                        height: req.height,
                        proof_ops,
                        ..Default::default()
                    }
                },
                Err(err) => ResponseQuery {
                    code: 1,
                    codespace: "store".into(),
                    log: err.to_string(),
                    ..Default::default()
                },
            },
            unknown => ResponseQuery {
                code: 1,
                codespace: "app".into(),
                log: format!("unknown path `{unknown}`; must be `/app`, `/simulate`, or `/store`"),
                ..Default::default()
            },
        }
    }
}

fn from_tm_block(height: i64, time: Option<TmTimestamp>, hash: Option<Bytes>) -> BlockInfo {
    BlockInfo {
        height: height as u64,
        timestamp: from_tm_timestamp(time.expect("block time not found")),
        hash: hash.map(from_tm_hash).unwrap_or(GENESIS_BLOCK_HASH),
    }
}

fn from_tm_timestamp(time: TmTimestamp) -> Timestamp {
    Timestamp::from_seconds(time.seconds as u128) + Duration::from_nanos(time.nanos as u128)
}

fn from_tm_hash(bytes: Bytes) -> Hash256 {
    bytes
        .as_ref()
        .try_into()
        .expect("incorrect block hash length")
}

fn into_tm_tx_result(outcome: TxOutcome) -> ExecTxResult {
    match outcome.result {
        GenericResult::Ok(_) => ExecTxResult {
            code: 0,
            gas_wanted: outcome.gas_limit as i64,
            gas_used: outcome.gas_used as i64,
            events: into_tm_events(outcome.events),
            ..Default::default()
        },
        GenericResult::Err(err) => ExecTxResult {
            code: 1,
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
        r#type: event.r#type,
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
    TmAttribute {
        key: attr.key,
        value: attr.value,
        index: true,
    }
}
