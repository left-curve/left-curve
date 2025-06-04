use {
    grug_app::{AppError, AppResult, Db, Indexer, ProposalPreparer, Vm},
    grug_types::{CheckTxOutcome, Tx},
    std::sync::Arc,
};

pub type AppRef = Arc<dyn App>;

pub trait App: Send + Sync + 'static {
    fn check_tx(&self, tx: Tx) -> AppResult<CheckTxOutcome>;
}

impl<DB, VM, PP, ID> App for grug_app::App<DB, VM, PP, ID>
where
    DB: Db + Send + Sync + 'static,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer + Send + Sync + 'static,
    ID: Indexer + Send + Sync + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn check_tx(&self, tx: Tx) -> AppResult<CheckTxOutcome> {
        self.do_check_tx(tx)
    }
}
