use {
    grug_app::{
        App, AppError, AppResult, CHAIN_ID, Db, Indexer, LAST_FINALIZED_BLOCK, ProposalPreparer, Vm,
    },
    grug_testing::TestSuite,
    grug_types::{BlockInfo, BroadcastClient, JsonDeExt, JsonSerExt, SearchTxClient},
    std::sync::Arc,
    tokio::runtime::Runtime,
};

pub trait QueryApp {
    /// Query the app, return a JSON String.
    fn query_app(&self, raw_req: String, height: Option<u64>) -> AppResult<String>;

    /// Query the app's underlying key-value store, return `(value, proof)`.
    fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)>;

    /// Simulate a transaction, return a JSON String.
    fn simulate(&self, raw_unsigned_tx: String) -> AppResult<String>;

    /// Query the chain ID.
    fn chain_id(&self) -> AppResult<String>;

    /// Query the last finalized block.
    fn last_finalized_block(&self) -> AppResult<BlockInfo>;
}

impl<DB, VM, PP, ID> QueryApp for App<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn query_app(&self, raw_req: String, height: Option<u64>) -> AppResult<String> {
        let req = raw_req.deserialize_json()?;
        let res = self.do_query_app(req, height.unwrap_or(0), false)?;

        Ok(res.to_json_string()?)
    }

    fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        self.do_query_store(key, height.unwrap_or(0), prove)
    }

    fn simulate(&self, raw_unsigned_tx: String) -> AppResult<String> {
        let tx = raw_unsigned_tx.as_bytes().deserialize_json()?;
        let res = self.do_simulate(tx, 0, false)?;

        Ok(res.to_json_string()?)
    }

    fn chain_id(&self) -> AppResult<String> {
        let storage = self.db.state_storage(None)?;
        let chain_id = CHAIN_ID.load(&storage)?;

        Ok(chain_id)
    }

    fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        let storage = self.db.state_storage(None)?;
        let last_finalized_block = LAST_FINALIZED_BLOCK.load(&storage)?;

        Ok(last_finalized_block)
    }
}

impl<T> QueryApp for Arc<tokio::sync::Mutex<T>>
where
    T: QueryApp + Send + Sync + 'static,
{
    fn query_app(&self, raw_req: String, height: Option<u64>) -> AppResult<String> {
        let arc_clone = Arc::clone(self);

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let guard = arc_clone.lock().await;
                guard.query_app(raw_req, height)
            })
        })
        .join()
        .unwrap()
    }

    fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let arc_clone = Arc::clone(self);
        let key_clone = key.to_vec();

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let guard = arc_clone.lock().await;
                guard.query_store(&key_clone, height, prove)
            })
        })
        .join()
        .unwrap()
    }

    fn simulate(&self, raw_unsigned_tx: String) -> AppResult<String> {
        let arc_clone = Arc::clone(self);

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let guard = arc_clone.lock().await;
                guard.simulate(raw_unsigned_tx)
            })
        })
        .join()
        .unwrap()
    }

    fn chain_id(&self) -> AppResult<String> {
        let arc_clone = Arc::clone(self);

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let guard = arc_clone.lock().await;
                guard.chain_id()
            })
        })
        .join()
        .unwrap()
    }

    fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        let arc_clone = Arc::clone(self);

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let guard = arc_clone.lock().await;
                guard.last_finalized_block()
            })
        })
        .join()
        .unwrap()
    }
}

impl<DB, VM, PP, ID> QueryApp for TestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm,
    PP: ProposalPreparer,
    ID: Indexer,
    App<DB, VM, PP, ID>: QueryApp,
{
    fn query_app(&self, raw_req: String, height: Option<u64>) -> AppResult<String> {
        self.app.query_app(raw_req, height)
    }

    fn query_store(
        &self,
        key: &[u8],
        height: Option<u64>,
        prove: bool,
    ) -> AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        self.app.query_store(key, height, prove)
    }

    fn simulate(&self, raw_unsigned_tx: String) -> AppResult<String> {
        self.app.simulate(raw_unsigned_tx)
    }

    fn chain_id(&self) -> AppResult<String> {
        self.app.chain_id()
    }

    fn last_finalized_block(&self) -> AppResult<BlockInfo> {
        self.app.last_finalized_block()
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
