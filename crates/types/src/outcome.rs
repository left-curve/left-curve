use {
    grug_types::{Event, GenericResult, Hash},
    serde::{Deserialize, Serialize},
};

/// Outcome of executing a block.
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<GenericResult<Outcome>>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<GenericResult<Outcome>>,
}

/// Outcome of executing a single message, transaction, or cronjob.
///
/// Includes the events emitted, and gas consumption.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    // `None` if the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub events: Vec<Event>,
}

impl Outcome {
    pub fn new(gas_limit: Option<u64>) -> Self {
        Self {
            gas_limit,
            gas_used: 0,
            events: vec![],
        }
    }

    pub fn add_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    /// In case we make two calls consecutively (e.g. first `bank_transfer`,
    /// `receive`), merge the second outcome into the first one.
    pub fn update(&mut self, other: Outcome) {
        // The two calls should share the same gas tracker, so the limit should
        // be the same.
        debug_assert_eq!(self.gas_limit, other.gas_limit, "Gas limit somehow changed");

        self.gas_used = other.gas_used;
        self.events.extend(other.events);
    }
}
