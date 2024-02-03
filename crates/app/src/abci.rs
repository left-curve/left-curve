use {
    crate::{App, AppResult},
    cw_std::{Attribute, BlockInfo, Event, Hash, Storage, Timestamp, Uint64},
    std::net::ToSocketAddrs,
    tendermint_abci::{Application, Error as ABCIError, ServerBuilder},
    tendermint_proto::{
        abci::{
            Event as TmEvent, EventAttribute as TmAttribute, ExecTxResult, RequestCheckTx,
            RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestQuery, ResponseCheckTx,
            ResponseCommit, ResponseFinalizeBlock, ResponseInfo, ResponseInitChain, ResponseQuery,
        },
        google::protobuf::Timestamp as TmTimestamp,
    },
    tracing::Value,
};

impl<S> App<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub fn start_abci_server(
        self,
        read_buf_size: usize,
        addr: impl ToSocketAddrs + Value,
    ) -> Result<(), ABCIError> {
        ServerBuilder::new(read_buf_size)
            .bind(addr, self)?
            .listen()
    }
}

impl<S> Application for App<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    fn info(&self, _req: RequestInfo) -> ResponseInfo {
        match self.do_info() {
            Ok((last_block_height, last_block_version)) => {
                ResponseInfo {
                    data:                env!("CARGO_PKG_NAME").into(),
                    version:             env!("CARGO_PKG_VERSION").into(),
                    app_version:         1,
                    last_block_app_hash: last_block_version.into_vec().into(),
                    last_block_height:   last_block_height.u64() as i64,
                }
            },
            Err(err) => panic!("failed to get info: {err}"),
        }
    }

    fn init_chain(&self, req: RequestInitChain) -> ResponseInitChain {
        let block = from_tm_block(req.initial_height, req.time);

        match self.do_init_chain(req.chain_id, block, &req.app_state_bytes) {
            Ok(app_hash) => {
                ResponseInitChain {
                    consensus_params: req.consensus_params,
                    validators:       req.validators,
                    app_hash:         app_hash.into_vec().into(),
                }
            },
            Err(err) => panic!("failed to init chain: {err}"),
        }
    }

    fn finalize_block(&self, req: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let block = from_tm_block(req.height, req.time);

        match self.do_finalize_block(block, req.txs) {
            Ok(tx_results) => {
                ResponseFinalizeBlock {
                    events:                  vec![], // this should be begin/endblocker events, which we don't have yet
                    tx_results:              tx_results.into_iter().map(to_tm_tx_result).collect(),
                    validator_updates:       vec![],
                    consensus_param_updates: None,
                    app_hash:                Hash::zero().into_vec().into(),
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

    fn check_tx(&self, _req: RequestCheckTx) -> ResponseCheckTx {
        // TODO
        ResponseCheckTx {
            ..Default::default()
        }
    }

    // TODO: From ABCI docs (https://github.com/cometbft/cometbft/blob/main/spec/abci/abci++_methods.md):
    //
    // > Applications MUST interpret "/store" or any path starting with "/store/"
    // > as a query by key on the underlying store, in which case a key SHOULD
    // > be specified in data. Applications SHOULD allow queries over specific
    // > types like /accounts/... or /votes/....
    //
    // Currently we're going neither of these. We ignore `path`, `height`, and
    // `prove` fields, and interpret `data` as a JSON-encoded QueryRequest.
    fn query(&self, req: RequestQuery) -> ResponseQuery {
        match req.path.as_str() {
            "/app" => match self.do_query_app(&req.data) {
                Ok(res) => {
                    ResponseQuery {
                        code:  0,
                        value: res.to_vec().into(),
                        // TODO: more fields...
                        ..Default::default()
                    }
                },
                Err(err) => {
                    ResponseQuery {
                        code:      1,            // TODO: custom error code
                        codespace: "app".into(), // TODO: custom error codespace
                        log:       err.to_string(),
                        ..Default::default()
                    }
                },
            },
            "/store" => {
                let maybe_value = self.do_query_store(&req.data, req.height as u64, req.prove);
                ResponseQuery {
                    code:      0,
                    value:     maybe_value.unwrap_or_default().into(),
                    height:    req.height,
                    proof_ops: None, // TODO
                    ..Default::default()
                }
            }
            unknown => {
                ResponseQuery {
                    code:      1,
                    codespace: "app".into(),
                    log:       format!("unknown path `{unknown}`; must be `/app` or `/store`"),
                    ..Default::default()
                }
            }
        }

    }
}

fn from_tm_block(height: i64, time: Option<TmTimestamp>) -> BlockInfo {
    BlockInfo {
        height:    Uint64::new(height as u64),
        timestamp: from_tm_timestamp(time.expect("block time not found")),
    }
}

fn from_tm_timestamp(time: TmTimestamp) -> Timestamp {
    Timestamp::from_seconds(time.seconds as u64).plus_nanos(time.nanos as u64)
}

fn to_tm_tx_result(tx_result: AppResult<Vec<Event>>) -> ExecTxResult {
    match tx_result {
        Ok(events) => ExecTxResult {
            code:   0,
            events: events.into_iter().map(to_tm_event).collect(),
            ..Default::default()
        },
        Err(err) => ExecTxResult {
            code:      1,                // TODO: custom error code
            codespace: "tx".to_string(), // TODO: custom error codespace
            log:       err.to_string(),
            ..Default::default()
        },
    }
}

fn to_tm_event(event: Event) -> TmEvent {
    TmEvent {
        r#type:     event.r#type,
        attributes: event.attributes.into_iter().map(to_tm_attribute).collect(),
    }
}

fn to_tm_attribute(attr: Attribute) -> TmAttribute {
    TmAttribute {
        key:   attr.key,
        value: attr.value,
        index: true,
    }
}
