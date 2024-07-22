use {
    crate::{App, AppError, Db, Outcome, Vm},
    grug_types::{
        to_json_vec, Attribute, BlockInfo, Duration, Event, GenericResult, Hash, Timestamp, Uint64,
        GENESIS_BLOCK_HASH,
    },
    prost::bytes::Bytes,
    std::{any::type_name, net::ToSocketAddrs},
    tendermint_abci::{Application, Error as ABCIError, ServerBuilder},
    tendermint_proto::{
        abci::{
            Event as TmEvent, EventAttribute as TmAttribute, ExecTxResult, RequestCheckTx,
            RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestQuery, ResponseCheckTx,
            ResponseCommit, ResponseFinalizeBlock, ResponseInfo, ResponseInitChain, ResponseQuery,
        },
        crypto::{ProofOp, ProofOps},
        google::protobuf::Timestamp as TmTimestamp,
    },
};

impl<DB, VM> App<DB, VM>
where
    DB: Db + Clone + Send + 'static,
    VM: Vm + Clone + Send + 'static,
    AppError: From<DB::Error> + From<VM::Error>,
{
    pub fn start_abci_server<A>(self, read_buf_size: usize, addr: A) -> Result<(), ABCIError>
    where
        A: ToSocketAddrs,
    {
        ServerBuilder::new(read_buf_size).bind(addr, self)?.listen()
    }
}

impl<DB, VM> Application for App<DB, VM>
where
    DB: Db + Clone + Send + 'static,
    VM: Vm + Clone + Send + 'static,
    AppError: From<DB::Error> + From<VM::Error>,
{
    fn info(&self, _req: RequestInfo) -> ResponseInfo {
        match self.do_info() {
            Ok((last_block_height, last_block_version)) => ResponseInfo {
                data: env!("CARGO_PKG_NAME").into(),
                version: env!("CARGO_PKG_VERSION").into(),
                app_version: 1,
                last_block_app_hash: last_block_version.into_vec().into(),
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
                app_hash: app_hash.into_vec().into(),
            },
            Err(err) => panic!("failed to init chain: {err}"),
        }
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
                    .filter_map(|res| res.ok().map(|outcome| into_tm_events(outcome.events)))
                    .flatten()
                    .collect();

                let tx_results = outcome
                    .tx_outcomes
                    .into_iter()
                    .map(into_tm_tx_result)
                    .collect();

                ResponseFinalizeBlock {
                    app_hash: outcome.app_hash.into_vec().into(),
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
        match self.do_simulate_raw(&req.tx, 0, false) {
            Ok(outcome) => ResponseCheckTx {
                code: 0,
                // Again, really poor design of Tendermint...
                // 1. No null value (we have to use `0` to mean no gas limit)
                // 2. Why `i64` for gas? Who uses negative gas?
                gas_wanted: outcome.gas_limit.unwrap_or(0) as i64,
                gas_used: outcome.gas_used as i64,
                events: into_tm_events(outcome.events),
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
            "/simulate" => match self
                .do_simulate_raw(&req.data, req.height as u64, req.prove)
                .and_then(|outcome| to_json_vec(&outcome).map_err(Into::into))
            {
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
        height: Uint64::new(height as u64),
        timestamp: from_tm_timestamp(time.expect("block time not found")),
        hash: hash.map(from_tm_hash).unwrap_or(GENESIS_BLOCK_HASH),
    }
}

fn from_tm_timestamp(time: TmTimestamp) -> Timestamp {
    Timestamp::from_seconds(time.seconds as u128) + Duration::from_nanos(time.nanos as u128)
}

fn from_tm_hash(bytes: Bytes) -> Hash {
    bytes
        .to_vec()
        .try_into()
        .expect("incorrect block hash length")
}

fn into_tm_tx_result(tx_result: GenericResult<Outcome>) -> ExecTxResult {
    match tx_result {
        GenericResult::Ok(outcome) => ExecTxResult {
            code: 0,
            events: into_tm_events(outcome.events),
            ..Default::default()
        },
        GenericResult::Err(err) => ExecTxResult {
            code: 1,
            codespace: "tx".to_string(),
            log: err.to_string(),
            ..Default::default()
        },
    }
}

fn into_tm_events(events: Vec<Event>) -> Vec<TmEvent> {
    events.into_iter().map(into_tm_event).collect()
}

fn into_tm_event(event: Event) -> TmEvent {
    TmEvent {
        r#type: event.r#type,
        attributes: event
            .attributes
            .into_iter()
            .map(into_tm_attribute)
            .collect(),
    }
}

fn into_tm_attribute(attr: Attribute) -> TmAttribute {
    TmAttribute {
        key: attr.key,
        value: attr.value,
        index: true,
    }
}
