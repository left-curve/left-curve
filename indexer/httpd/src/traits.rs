#[cfg(feature = "testing")]
use grug_testing::TestSuite;
use {
    async_trait::async_trait,
    grug_app::{
        App, AppError, AppResult, CHAIN_ID, Db, Indexer, LAST_FINALIZED_BLOCK, ProposalPreparer, Vm,
    },
    grug_types::{
        BlockInfo, BroadcastClient, JsonDeExt, Query, QueryResponse, SearchTxClient, TxOutcome,
    },
};

#[async_trait]
pub trait QueryApp {
    /// Query the app, return a JSON String.
    async fn query_app(
        &self,
        raw_req: serde_json::Value,
        height: Option<u64>,
    ) -> AppResult<QueryResponse>;

    /// Query the app's underlying key-value store, return `(value, proof)`.
    async fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)>;

    /// Simulate a transaction.
    async fn simulate(&self, raw_unsigned_tx: serde_json::Value) -> AppResult<TxOutcome>;

    /// Query the chain ID.
    async fn chain_id(&self) -> AppResult<String>;

    /// Query the last finalized block.
    async fn last_finalized_block(&self) -> AppResult<BlockInfo>;
}

#[async_trait]
impl<DB, VM, PP, ID> QueryApp for App<DB, VM, PP, ID>
where
    DB: Db + Send + Sync + 'static,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer + Send + Sync + 'static,
    ID: Indexer + Send + Sync + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    async fn query_app(
        &self,
        raw_req: serde_json::Value,
        height: Option<u64>,
    ) -> AppResult<QueryResponse> {
        let req: Query = raw_req.to_string().deserialize_json()?;

        Ok(self.do_query_app(req, height.unwrap_or(0), false)?)
    }

    async fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        self.do_query_store(key, height.unwrap_or(0), prove)
    }

    async fn simulate(&self, raw_unsigned_tx: serde_json::Value) -> AppResult<TxOutcome> {
        let tx = raw_unsigned_tx.to_string().deserialize_json()?;

        Ok(self.do_simulate(tx, 0, false)?)
    }

    async fn chain_id(&self) -> AppResult<String> {
        let storage = self.db.state_storage(None)?;
        let chain_id = CHAIN_ID.load(&storage)?;

        Ok(chain_id)
    }

    async fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        let storage = self.db.state_storage(None)?;
        let last_finalized_block = LAST_FINALIZED_BLOCK.load(&storage)?;

        Ok(last_finalized_block)
    }
}

#[cfg(feature = "testing")]
#[async_trait]
impl<DB, VM, PP, ID> QueryApp for TestSuite<DB, VM, PP, ID>
where
    DB: Db + Send + Sync + 'static,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer + Send + Sync + 'static,
    ID: Indexer + Send + Sync + 'static,
    App<DB, VM, PP, ID>: QueryApp,
{
    async fn query_app(
        &self,
        raw_req: serde_json::Value,
        height: Option<u64>,
    ) -> AppResult<QueryResponse> {
        self.app.query_app(raw_req, height).await
    }

    async fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        self.app.query_store(key, height, prove).await
    }

    async fn simulate(&self, raw_unsigned_tx: serde_json::Value) -> AppResult<TxOutcome> {
        self.app.simulate(raw_unsigned_tx).await
    }

    async fn chain_id(&self) -> AppResult<String> {
        self.app.chain_id().await
    }

    async fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        self.app.last_finalized_block().await
    }
}

#[async_trait]
impl<T> QueryApp for tokio::sync::Mutex<T>
where
    T: QueryApp + Send + Sync + 'static,
{
    async fn query_app(
        &self,
        raw_req: serde_json::Value,
        height: Option<u64>,
    ) -> AppResult<QueryResponse> {
        self.lock().await.query_app(raw_req, height).await
    }

    async fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        self.lock().await.query_store(key, height, prove).await
    }

    async fn simulate(&self, raw_unsigned_tx: serde_json::Value) -> AppResult<TxOutcome> {
        self.lock().await.simulate(raw_unsigned_tx).await
    }

    async fn chain_id(&self) -> AppResult<String> {
        self.lock().await.chain_id().await
    }

    async fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        self.lock().await.last_finalized_block().await
    }
}

pub trait ConsensusClient:
    SearchTxClient<Error = anyhow::Error> + BroadcastClient<Error = anyhow::Error>
{
}

impl<T> ConsensusClient for T where
    T: SearchTxClient<Error = anyhow::Error> + BroadcastClient<Error = anyhow::Error>
{
}
