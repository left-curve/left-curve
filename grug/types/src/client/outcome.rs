use {
    crate::{CheckTxError, CheckTxOutcome, CheckTxSuccess, Hash256, ResultExt, Tx, TxOutcome},
    serde::Serialize,
};

#[derive(Serialize)]
pub struct BroadcastTxOutcome {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxOutcome,
}

pub struct BroadcastTxError {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxError,
}

pub struct BroadcastTxSuccess {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxSuccess,
}

impl BroadcastTxOutcome {
    #[allow(clippy::result_large_err)]
    pub fn into_result(self) -> Result<BroadcastTxSuccess, BroadcastTxError> {
        match &self.check_tx.result {
            Ok(_) => Ok(BroadcastTxSuccess {
                tx_hash: self.tx_hash,
                check_tx: self.check_tx.should_succeed(),
            }),
            Err(_) => Err(BroadcastTxError {
                tx_hash: self.tx_hash,
                check_tx: self.check_tx.should_fail(),
            }),
        }
    }
}

#[derive(Serialize)]
pub struct SearchTxOutcome {
    pub hash: Hash256,
    pub height: u64,
    pub index: u32,
    pub tx: Tx,
    pub outcome: TxOutcome,
}
