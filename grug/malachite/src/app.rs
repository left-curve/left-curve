use {
    crate::types::{AppHash, RawTx},
    grug::{BlockInfo, CheckTxOutcome, JsonDeExt},
    grug_app::{AppError, AppResult, Db, Indexer, ProposalPreparer, Vm},
    std::sync::Arc,
};

pub type MempoolAppRef = Arc<dyn MempoolApp>;

pub trait MempoolApp: Send + Sync + 'static {
    fn check_tx(&self, tx: &RawTx) -> AppResult<CheckTxOutcome>;
}

impl<DB, VM, PP, ID> MempoolApp for grug_app::App<DB, VM, PP, ID>
where
    DB: Db + Send + Sync + 'static,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer + Send + Sync + 'static,
    ID: Indexer + Send + Sync + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn check_tx(&self, tx: &RawTx) -> AppResult<CheckTxOutcome> {
        self.do_check_tx(tx.deserialize_json()?)
    }
}

pub type HostAppRef = Arc<dyn HostApp>;
pub trait HostApp: Send + Sync + 'static {
    fn prepare_proposal(&self, txs: Vec<RawTx>) -> Vec<RawTx>;
    fn finalize_block(&self, block: BlockInfo, txs: &[RawTx]) -> AppResult<AppHash>;
    fn commit(&self) -> AppResult<()>;
}

impl<DB, VM, PP, ID> HostApp for grug_app::App<DB, VM, PP, ID>
where
    DB: Db + Send + Sync + 'static,
    VM: Vm + Clone + Send + Sync + 'static,
    PP: ProposalPreparer + Send + Sync + 'static,
    ID: Indexer + Send + Sync + 'static,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn prepare_proposal(&self, txs: Vec<RawTx>) -> Vec<RawTx> {
        // TODO: This need to be optimized, probably the best solution is to change to perpare proposal function signature
        self.do_prepare_proposal(txs.into_iter().map(|tx| tx.0).collect(), usize::MAX)
            .into_iter()
            .map(RawTx)
            .collect()
    }

    fn finalize_block(&self, block: BlockInfo, txs: &[RawTx]) -> AppResult<AppHash> {
        self.do_finalize_block_raw(block, &txs)
            .map(|outcome| AppHash::new(outcome.app_hash))
    }

    fn commit(&self) -> AppResult<()> {
        self.do_commit()
    }
}
