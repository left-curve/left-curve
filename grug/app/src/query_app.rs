use {
    crate::{App, AppError, AppResult, Db, Indexer, ProposalPreparer, Vm},
    grug_types::{JsonDeExt, JsonSerExt},
};

pub trait QueryApp {
    fn query_app(&self, raw_req: String, height: u64, prove: bool) -> AppResult<String>;
    fn simulate(&self, raw_unsigned_tx: String, height: u64, prove: bool) -> AppResult<String>;
}

impl<DB, VM, PP, ID> QueryApp for App<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    /// Query the app, return a JSON String
    fn query_app(&self, raw_req: String, height: u64, prove: bool) -> AppResult<String> {
        let req = raw_req.deserialize_json()?;
        let res = self.do_query_app(req, height, prove)?;

        Ok(res.to_json_string()?)
    }

    /// Simulate a transaction, return a JSON String
    fn simulate(&self, raw_unsigned_tx: String, height: u64, prove: bool) -> AppResult<String> {
        let tx = raw_unsigned_tx.as_bytes().deserialize_json()?;
        let res = self.do_simulate(tx, height, prove)?;

        Ok(res.to_json_string()?)
    }
}
