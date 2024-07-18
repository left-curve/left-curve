use {
    crate::{App, AppError, AppResult, Db, Vm},
    grug_types::{Attribute, BlockInfo, Event, Hash, Timestamp, Uint64, GENESIS_BLOCK_HASH},
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
        // ignore req.initial_height. we always consider the block height during
        // InitChain to be zero. this is necessary to make sure BaseStore version
        // always matches block height.
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
            Ok((app_hash, events, tx_results)) => ResponseFinalizeBlock {
                events: events.into_iter().map(to_tm_event).collect(),
                tx_results: tx_results.into_iter().map(to_tm_tx_result).collect(),
                validator_updates: vec![],
                consensus_param_updates: None,
                app_hash: app_hash.into_vec().into(),
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

    fn check_tx(&self, _req: RequestCheckTx) -> ResponseCheckTx {
        // TODO
        ResponseCheckTx {
            ..Default::default()
        }
    }

    fn query(&self, req: RequestQuery) -> ResponseQuery {
        match req.path.as_str() {
            "/app" => match self.do_query_app_raw(&req.data, req.height as u64, req.prove) {
                Ok(res) => ResponseQuery {
                    code: 0,
                    value: res.to_vec().into(),
                    ..Default::default()
                },
                Err(err) => ResponseQuery {
                    code: 1,
                    codespace: "app".into(),
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
                log: format!("unknown path `{unknown}`; must be `/app` or `/store`"),
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
    Timestamp::from_seconds(time.seconds as u128).plus_nanos(time.nanos as u128)
}

fn from_tm_hash(bytes: Bytes) -> Hash {
    bytes
        .to_vec()
        .try_into()
        .expect("incorrect block hash length")
}

fn to_tm_tx_result(tx_result: AppResult<Vec<Event>>) -> ExecTxResult {
    match tx_result {
        Ok(events) => ExecTxResult {
            code: 0,
            events: events.into_iter().map(to_tm_event).collect(),
            ..Default::default()
        },
        Err(err) => ExecTxResult {
            code: 1,                     // TODO: custom error code
            codespace: "tx".to_string(), // TODO: custom error codespace
            log: err.to_string(),
            ..Default::default()
        },
    }
}

fn to_tm_event(event: Event) -> TmEvent {
    TmEvent {
        r#type: event.r#type,
        attributes: event.attributes.into_iter().map(to_tm_attribute).collect(),
    }
}

fn to_tm_attribute(attr: Attribute) -> TmAttribute {
    TmAttribute {
        key: attr.key,
        value: attr.value,
        index: true,
    }
}
