use grug_types::{Addr, BlockOutcome, Hash256, ResultExt, Tx, TxError, TxOutcome, TxSuccess};

#[must_use = "`UploadOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct UploadOutcome {
    pub(crate) code_hash: Hash256,
    pub(crate) outcome: TxOutcome,
}

pub struct UploadOutcomeSuccess {
    pub code_hash: Hash256,
    pub outcome: TxSuccess,
}

impl ResultExt for UploadOutcome {
    type Error = TxError;
    type Success = UploadOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        UploadOutcomeSuccess {
            code_hash: self.code_hash,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

#[must_use = "`InstantiateOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct InstantiateOutcome {
    pub(crate) address: Addr,
    pub(crate) outcome: TxOutcome,
}

pub struct InstantiateOutcomeSuccess {
    pub address: Addr,
    pub outcome: TxSuccess,
}

impl ResultExt for InstantiateOutcome {
    type Error = TxError;
    type Success = InstantiateOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        InstantiateOutcomeSuccess {
            address: self.address,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

#[must_use = "`UploadAndInstantiateOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct UploadAndInstantiateOutcome {
    pub(crate) code_hash: Hash256,
    pub(crate) address: Addr,
    pub(crate) outcome: TxOutcome,
}

pub struct UploadAndInstantiateOutcomeSuccess {
    pub address: Addr,
    pub code_hash: Hash256,
    pub outcome: TxSuccess,
}

impl ResultExt for UploadAndInstantiateOutcome {
    type Error = TxError;
    type Success = UploadAndInstantiateOutcomeSuccess;

    fn should_succeed(self) -> Self::Success {
        UploadAndInstantiateOutcomeSuccess {
            address: self.address,
            code_hash: self.code_hash,
            outcome: self.outcome.should_succeed(),
        }
    }

    fn should_fail(self) -> Self::Error {
        self.outcome.should_fail()
    }
}

pub struct MakeBlockOutcome {
    pub txs: Vec<(Tx, Hash256)>,
    pub block_outcome: BlockOutcome,
}
