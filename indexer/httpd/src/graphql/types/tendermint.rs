use {
    async_graphql::SimpleObject,
    tendermint_rpc::endpoint::{
        abci_query,
        broadcast::{tx_async, tx_commit, tx_sync},
    },
};

#[derive(SimpleObject)]
pub struct TxSyncResponse {
    pub code: u32,
    pub data: Vec<u8>,
    pub log: String,
    pub hash: String,
}

impl From<tx_sync::Response> for TxSyncResponse {
    fn from(resp: tx_sync::Response) -> Self {
        TxSyncResponse {
            code: resp.code.value(),
            data: resp.data.into(),
            log: resp.log,
            hash: resp.hash.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub struct TxAsyncResponse {
    pub codespace: String,
    pub code: u32,
    pub data: Vec<u8>,
    pub log: String,
    pub hash: String,
}

impl From<tx_async::Response> for TxAsyncResponse {
    fn from(resp: tx_async::Response) -> Self {
        TxAsyncResponse {
            codespace: resp.codespace,
            code: resp.code.value(),
            data: resp.data.into(),
            log: resp.log,
            hash: resp.hash.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub struct TxCommitResponse {
    pub hash: String,
    pub block_height: u64,
}

impl From<tx_commit::Response> for TxCommitResponse {
    fn from(resp: tx_commit::Response) -> Self {
        TxCommitResponse {
            hash: resp.hash.to_string(),
            block_height: resp.height.into(),
        }
    }
}

#[derive(SimpleObject)]
pub struct AbciQuery {
    pub log: String,
    pub code: u32,
    pub info: String,
    pub index: i64,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub height: u64,
    pub codespace: String,
}

impl From<abci_query::AbciQuery> for AbciQuery {
    fn from(query: abci_query::AbciQuery) -> Self {
        AbciQuery {
            log: query.log,
            code: query.code.into(),
            info: query.info,
            index: query.index,
            key: query.key,
            value: query.value,
            height: query.height.into(),
            codespace: query.codespace,
        }
    }
}
