use {
    crate::{Event, GenericResult, Hash},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// Outcome of executing one or more messages or cronjobs.
///
/// Includes the events emitted, and gas consumption.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    // `None` means the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub result: GenericResult<Vec<Event>>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TxOutcome {
    /// Outcome of the first three steps in the transaction handling flow.
    ///
    /// That is,
    ///
    /// 1. Call the sender's `before_tx` function
    /// 2. Execute the messages
    /// 3. Call the sender's `after_tx` function
    pub msg_outcome: Outcome,
    /// Outcome of the fourth and final step in the transaction handling flow.
    ///
    /// That is,
    ///
    /// 4. Call the taxman's `handle_fee` function
    pub tax_outcome: Outcome,
}

impl TxOutcome {
    pub fn should_succeed(&self) {
        match (&self.msg_outcome.result, &self.tax_outcome.result) {
            (GenericResult::Err(err), _) | (_, GenericResult::Err(err)) => {
                panic!("expecting ok, got err: {err}");
            },
            _ => (),
        }
    }

    pub fn should_fail(&self) {
        match (&self.msg_outcome.result, &self.tax_outcome.result) {
            (GenericResult::Ok(_), GenericResult::Ok(_)) => {
                panic!("expecting err, got ok");
            },
            _ => (),
        }
    }

    pub fn should_fail_with_error<M>(&self, msg: M)
    where
        M: ToString,
    {
        match (&self.msg_outcome.result, &self.tax_outcome.result) {
            (GenericResult::Ok(_), GenericResult::Ok(_)) => {
                panic!("expecting err, got ok");
            },
            // Tax error takes precedence over msg error
            (_, GenericResult::Err(err)) | (GenericResult::Err(err), _) => {
                assert_eq!(
                    *err,
                    msg.to_string(),
                    "error as expected, but for wrong reason: {err}"
                );
            },
        }
    }
}

/// Outcome of executing a block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<Outcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<TxOutcome>,
}
