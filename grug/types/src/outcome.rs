use {
    crate::{
        CommitmentStatus, Event, EventStatus, EvtAuthenticate, EvtBackrun, EvtCron, EvtFinalize,
        EvtWithhold, GenericResult, Hash256, ResultExt, StdError, Tx,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    data_encoding::{BASE64_NOPAD, HEXUPPER},
    serde::{Deserialize, Serialize},
    std::fmt::{self, Display},
};
#[cfg(feature = "tendermint")]
use {
    crate::{JsonDeExt, StdResult},
    data_encoding::BASE64,
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

#[cfg(feature = "tendermint")]
impl CronOutcome {
    pub fn from_tm_event(tm_event: tendermint::abci::Event) -> StdResult<Self> {
        tm_event
            .attributes
            .first()
            .unwrap()
            .value_bytes()
            .deserialize_json()
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

#[cfg(feature = "tendermint")]
impl TxOutcome {
    pub fn from_tm_tx_result(
        tm_tx_result: tendermint::abci::types::ExecTxResult,
    ) -> StdResult<Self> {
        Ok(Self {
            gas_limit: tm_tx_result.gas_wanted as u64,
            gas_used: tm_tx_result.gas_used as u64,
            result: into_generic_result(tm_tx_result.code, tm_tx_result.log),
            events: BASE64.decode(&tm_tx_result.data)?.deserialize_json()?,
        })
    }
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

    #[cfg(feature = "tendermint")]
    pub fn from_tm_data(raw_bytes: &[u8]) -> StdResult<Self> {
        let b64 = BASE64_NOPAD.encode(&raw_bytes);
        let hex = HEXUPPER.decode(b64.as_bytes())?;

        // remove all bytes after the last }
        let end = hex.iter().rposition(|&b| b == b'}').unwrap_or(hex.len());
        let mut bytes = if end == hex.len() {
            hex.to_vec()
        } else {
            hex[..=end].to_vec()
        };

        let curly_open = bytes.iter().filter(|b| **b == b'{').count();
        let curly_close = bytes.iter().filter(|b| **b == b'}').count();

        let diff = curly_open.checked_sub(curly_close).ok_or(StdError::Math(
            grug_math::MathError::overflow_sub(curly_open, curly_close),
        ))?;

        // Add a } at the end of the string for each missing {
        for _ in 0..diff {
            bytes.push(b'}');
        }

        bytes.deserialize_json()
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

#[derive(Serialize)]
pub struct BroadcastTxOutcome {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxOutcome,
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

#[cfg(feature = "tendermint")]
impl BroadcastTxOutcome {
    pub fn from_tm_broadcast_response(
        response: tendermint_rpc::endpoint::broadcast::tx_sync::Response,
    ) -> StdResult<Self> {
        Ok(Self {
            tx_hash: Hash256::from_inner(response.hash.as_bytes().try_into()?),
            check_tx: CheckTxOutcome {
                gas_limit: 0,
                gas_used: 0,
                result: into_generic_result(response.code, response.log),
                // The data has a strange format.
                // If it fails to deserialize, we return a mock value.
                events: CheckTxEvents::from_tm_data(&response.data).unwrap_or(CheckTxEvents {
                    withhold: CommitmentStatus::NotReached,
                    authenticate: CommitmentStatus::NotReached,
                }),
            },
        })
    }
}

pub struct BroadcastTxError {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxError,
}

pub struct BroadcastTxSuccess {
    pub tx_hash: Hash256,
    pub check_tx: CheckTxSuccess,
}

#[derive(Serialize)]
pub struct SearchTxOutcome {
    pub hash: Hash256,
    pub height: u64,
    pub index: u32,
    pub tx: Tx,
    pub outcome: TxOutcome,
}

#[cfg(feature = "tendermint")]
impl SearchTxOutcome {
    pub fn from_tm_query_tx_response(
        response: tendermint_rpc::endpoint::tx::Response,
    ) -> StdResult<Self> {
        Ok(Self {
            hash: Hash256::from_inner(response.hash.as_bytes().try_into()?),
            height: response.height.into(),
            index: response.index,
            tx: BASE64.decode(&response.tx)?.deserialize_json()?,
            outcome: TxOutcome::from_tm_tx_result(response.tx_result)?,
        })
    }
}

#[cfg(feature = "tendermint")]
fn into_generic_result(code: tendermint::abci::Code, log: String) -> GenericResult<()> {
    if code == tendermint::abci::Code::Ok {
        Ok(())
    } else {
        Err(log)
    }
}
