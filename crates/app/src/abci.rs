use {
    crate::App,
    cw_db::{Flush, Storage},
    cw_std::{BlockInfo, Hash},
    tendermint_abci::Application,
    tendermint_proto::{
        abci::{
            RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestQuery,
            ResponseCheckTx, ResponseCommit, ResponseFinalizeBlock, ResponseInfo,
            ResponseInitChain, ResponseQuery,
        },
        google::protobuf::Timestamp,
    },
    tracing::{debug, trace},
};

impl<S> Application for App<S>
where
    S: Clone + Send + Sync + Storage + Flush + 'static,
{
    fn info(&self, req: RequestInfo) -> ResponseInfo {
        debug!(
            tm_version    = req.version,
            block_version = req.block_version,
            p2p_version   = req.p2p_version,
            abci_version  = req.abci_version,
            "got info request"
        );

        match self.do_info() {
            Ok((last_block_height, last_block_version)) => {
                ResponseInfo {
                    data:                env!("CARGO_PKG_NAME").into(),
                    version:             env!("CARGO_PKG_VERSION").into(),
                    app_version:         1,
                    last_block_app_hash: last_block_version.into_vec().into(),
                    last_block_height,
                }
            },
            Err(err) => panic!("failed to get info: {err}"),
        }
    }

    fn init_chain(&self, req: RequestInitChain) -> ResponseInitChain {
        debug!(
            chain_id = req.chain_id,
            height   = req.initial_height,
            time     = ?req.time,
            "got init chain request"
        );

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
        debug!(
            height  = req.height,
            time    = ?req.time,
            num_txs = req.txs.len(),
            hash    = hex::encode(&req.hash),
            "got finalize block request"
        );

        let block = from_tm_block(req.height, req.time);

        match self.do_finalize_block(block, req.txs) {
            Ok(()) => {
                ResponseFinalizeBlock {
                    events:                  vec![],
                    tx_results:              vec![],
                    validator_updates:       vec![],
                    consensus_param_updates: None,
                    app_hash:                Hash::zero().into_vec().into(),
                }
            },
            Err(err) => panic!("failed to finalize block: {err}"),
        }
    }

    fn commit(&self) -> ResponseCommit {
        debug!("got commit request");

        match self.do_commit() {
            Ok(()) => {
                ResponseCommit {
                    retain_height: 0, // TODO: what this means??
                }
            },
            Err(err) => panic!("failed to commit: {err}"),
        }
    }

    fn query(&self, req: RequestQuery) -> ResponseQuery {
        trace!("got query request");

        match self.do_query(&req.data) {
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
        }
    }

    fn check_tx(&self, _req: RequestCheckTx) -> ResponseCheckTx {
        trace!("got check tx request");

        // TODO
        ResponseCheckTx {
            ..Default::default()
        }
    }
}

fn from_tm_block(height: i64, time: Option<Timestamp>) -> BlockInfo {
    BlockInfo {
        height:    height as u64,
        timestamp: time.expect("block time not given").seconds as u64,
    }
}
