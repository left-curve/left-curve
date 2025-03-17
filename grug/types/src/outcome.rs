use {
    crate::{
        CommitmentStatus, Event, EventStatus, EvtAuthenticate, EvtBackrun, EvtCron, EvtFinalize,
        EvtWithhold, GenericResult, Hash256,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::fmt::{self, Display},
};

/// Outcome of performing an operation that is not a full tx. These include:
///
/// - processing a message;
/// - executing a cronjob;
/// - performing a `CheckTx` call.
///
/// Includes the events emitted, and gas consumption.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[must_use = "`Outcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct CheckTxOutcome {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub result: GenericResult<()>,
    pub events: CheckTxEvents,
}

/// The success case of [`TxOutcome`](crate::TxOutcome).
#[derive(Debug, PartialEq, Eq)]
pub struct CheckTxSuccess {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub events: CheckTxEvents,
}

/// The error case of [`TxOutcome`](crate::TxOutcome).
#[derive(Debug, PartialEq, Eq)]
pub struct CheckTxError {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub error: String,
    pub events: CheckTxEvents,
}

// `TxError` must implement `ToString`, such that it satisfies that trait bound
// required by `ResultExt::should_fail_with_error`.
impl Display for CheckTxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}",)
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[must_use = "`Outcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct CronOutcome {
    // `None` means the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub cron_event: CommitmentStatus<EventStatus<EvtCron>>,
}

impl CronOutcome {
    pub fn new(
        gas_limit: Option<u64>,
        gas_used: u64,
        cron_event: CommitmentStatus<EventStatus<EvtCron>>,
    ) -> Self {
        Self {
            gas_limit,
            gas_used,
            cron_event,
        }
    }
}

/// Outcome of processing a transaction.
///
/// Different from `Outcome`, which can either succeed or fail, a transaction
/// can partially succeed. A typical such scenario is:
///
/// - `withhold_fee` succeeds
/// - `authenticate` succeeds,
/// - one of the messages fail
/// - `finalize_fee` succeeds
///
/// In this case, state changes from fee handling (e.g. deducting the fee from
/// the sender account) and authentication (e.g. incrementing the sender account's
/// sequence number) will be committed, and relevant events emitted to reflect
/// this. However, state changes and events from the messages are discarded.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[must_use = "`TxOutcome` must be checked for success or error with `should_succeed`, `should_fail`, or similar methods."]
pub struct TxOutcome {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub result: GenericResult<()>,
    pub events: TxEvents,
}

/// The success case of [`TxOutcome`](crate::TxOutcome).
#[derive(Debug, PartialEq, Eq)]
pub struct TxSuccess {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub events: TxEvents,
}

/// The error case of [`TxOutcome`](crate::TxOutcome).
#[derive(Debug, PartialEq, Eq)]
pub struct TxError {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub error: String,
    pub events: TxEvents,
}

// `TxError` must implement `ToString`, such that it satisfies that trait bound
// required by `ResultExt::should_fail_with_error`.
impl Display for TxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}",)
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxEvents {
    pub withhold: CommitmentStatus<EventStatus<EvtWithhold>>,
    pub authenticate: CommitmentStatus<EventStatus<EvtAuthenticate>>,
    pub msgs_and_backrun: CommitmentStatus<MsgsAndBackrunEvents>,
    pub finalize: CommitmentStatus<EventStatus<EvtFinalize>>,
}

impl TxEvents {
    pub fn new(withhold: CommitmentStatus<EventStatus<EvtWithhold>>) -> Self {
        Self {
            withhold,
            authenticate: CommitmentStatus::NotReached,
            msgs_and_backrun: CommitmentStatus::NotReached,
            finalize: CommitmentStatus::NotReached,
        }
    }

    pub fn finalize_fails(
        self,
        finalize: CommitmentStatus<EventStatus<EvtFinalize>>,
        cause: &str,
    ) -> Self {
        fn update<T>(evt: CommitmentStatus<T>, cause: &str) -> CommitmentStatus<T> {
            if let CommitmentStatus::Committed(event) = evt {
                CommitmentStatus::Reverted {
                    event,
                    revert_by: cause.to_string(),
                }
            } else {
                evt
            }
        }

        TxEvents {
            withhold: update(self.withhold, cause),
            authenticate: update(self.authenticate, cause),
            msgs_and_backrun: update(self.msgs_and_backrun, cause),
            finalize,
        }
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CheckTxEvents {
    pub withhold: CommitmentStatus<EventStatus<EvtWithhold>>,
    pub authenticate: CommitmentStatus<EventStatus<EvtAuthenticate>>,
}

impl CheckTxEvents {
    pub fn new(withhold: CommitmentStatus<EventStatus<EvtWithhold>>) -> Self {
        Self {
            withhold,
            authenticate: CommitmentStatus::NotReached,
        }
    }
}
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgsAndBackrunEvents {
    pub msgs: Vec<EventStatus<Event>>, // len of the messages in this transaction
    pub backrun: EventStatus<EvtBackrun>,
}

impl MsgsAndBackrunEvents {
    pub fn base() -> Self {
        Self {
            msgs: vec![],
            backrun: EventStatus::NotReached,
        }
    }
}

impl Default for TxEvents {
    fn default() -> Self {
        Self {
            withhold: CommitmentStatus::NotReached,
            authenticate: CommitmentStatus::NotReached,
            msgs_and_backrun: CommitmentStatus::NotReached,
            finalize: CommitmentStatus::NotReached,
        }
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
/// Outcome of executing a block.
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash256,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<CronOutcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<TxOutcome>,
}
