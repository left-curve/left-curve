use {
    async_graphql::SimpleObject,
    base64::{Engine, engine::general_purpose::STANDARD},
    grug_math::Inner,
    grug_types::Binary,
    std::str::FromStr,
    tendermint_rpc::endpoint::{
        abci_query,
        broadcast::{tx_async, tx_commit, tx_sync},
    },
};

#[derive(SimpleObject)]
pub struct TxSyncResponse {
    pub code: u32,
    /// The base64 encoded data
    pub data: String,
    pub log: String,
    pub hash: String,
    pub codespace: String,
}

impl From<tx_sync::Response> for TxSyncResponse {
    fn from(resp: tx_sync::Response) -> Self {
        TxSyncResponse {
            code: resp.code.value(),
            data: STANDARD.encode(resp.data),
            log: resp.log,
            hash: resp.hash.to_string(),
            codespace: resp.codespace,
        }
    }
}

impl From<TxSyncResponse> for tx_sync::Response {
    fn from(resp: TxSyncResponse) -> Self {
        tx_sync::Response {
            code: tendermint::abci::Code::from(resp.code),
            codespace: resp.codespace,
            data: Binary::from_str(&resp.data).unwrap().into_inner().into(),
            log: resp.log,
            hash: tendermint::hash::Hash::from_str(&resp.hash).unwrap(),
        }
    }
}

#[derive(SimpleObject)]
pub struct TxAsyncResponse {
    pub codespace: String,
    pub code: u32,
    /// The base64 encoded data
    pub data: String,
    pub log: String,
    pub hash: String,
}

impl From<tx_async::Response> for TxAsyncResponse {
    fn from(resp: tx_async::Response) -> Self {
        TxAsyncResponse {
            codespace: resp.codespace,
            code: resp.code.value(),
            data: STANDARD.encode(resp.data),
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
    pub code: u32,
    pub log: String,
    pub info: String,
    pub index: i64,
    /// The base64 encoded key
    pub key: String,
    /// The base64 encoded value
    pub value: String,
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
            key: STANDARD.encode(query.key),
            value: STANDARD.encode(query.value),
            height: query.height.into(),
            codespace: query.codespace,
        }
    }
}
