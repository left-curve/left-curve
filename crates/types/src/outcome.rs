use {
    crate::{Event, GenericResult, Hash},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// Outcome of executing a single message, transaction, or cronjob.
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

/// Outcome of executing a block.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<Outcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<Outcome>,
}
