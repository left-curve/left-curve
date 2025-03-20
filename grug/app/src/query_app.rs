use {
    crate::{
        App, AppError, AppResult, CHAIN_ID, Db, Indexer, LAST_FINALIZED_BLOCK, ProposalPreparer, Vm,
    },
    grug_types::{BlockInfo, JsonDeExt, JsonSerExt},
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

    fn last_block(&self) -> AppResult<BlockInfo>;

    fn chain_id(&self) -> AppResult<String>;
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

    fn last_block(&self) -> AppResult<BlockInfo> {
        let storage = self.db.state_storage(None)?;

        Ok(LAST_FINALIZED_BLOCK.load(&storage)?)
    }

    fn chain_id(&self) -> AppResult<String> {
        let storage = self.db.state_storage(None)?;
        let chain_id = CHAIN_ID.load(&storage)?;

        Ok(chain_id)
    }
}
