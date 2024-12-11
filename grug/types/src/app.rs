use {
    crate::{
        Addr, CommitmentStatus, Duration, Event, EventStatus, EvtAuthenticate, EvtBackrun, EvtCron,
        EvtFinalize, EvtWithhold, GenericResult, Hash256, Json, Message, Timestamp,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    hex_literal::hex,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::{self, Display},
    },
};

/// The mock up sender address used for executing genesis messages.
///
/// Genesis messages aren't sent by a transaction, so don't actually have sender.
/// We use this as a mock up.
///
/// This is the RIPEMD-160 hash of the UTF-8 string `"sender"`.
pub const GENESIS_SENDER: Addr = Addr::from_inner(hex!("114af6e7a822df07328fba90e1546b5c2b00703f"));

/// The mock up block hash used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This is the SHA-256 hash of the UTF-8 string `"hash"`.
pub const GENESIS_BLOCK_HASH: Hash256 = Hash256::from_inner(hex!(
    "d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa"
));

/// The mock up block height used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This has to be zero, such as subsequent block heights are the same as the
/// database and Merkle tree version.
pub const GENESIS_BLOCK_HEIGHT: u64 = 0;

/// The chain's genesis state. To be included in the `app_state` field of
/// CometBFT's `genesis.json`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenesisState {
    /// Chain configurations.
    pub config: Config,
    /// App-specific configurations.
    pub app_config: Json,
    /// Messages to be executed in order during genesis.
    pub msgs: Vec<Message>,
}

/// Chain-level configurations. Not to be confused with contract-level configs.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The account that can update this config.
    pub owner: Addr,
    /// The contract the manages fungible token transfers.
    pub bank: Addr,
    /// The contract that handles transaction fees.
    pub taxman: Addr,
    /// A list of contracts that are to be called at regular time intervals.
    pub cronjobs: BTreeMap<Addr, Duration>,
    /// Permissions for certain gated actions.
    pub permissions: Permissions,
    /// Maximum age allowed for orphaned codes.
    /// A code is deleted if it remains orphaned (not used by any contract) for
    /// longer than this duration.
    pub max_orphan_age: Duration,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Permissions {
    pub upload: Permission,
    pub instantiate: Permission,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Permission {
    /// Only the owner can perform the action. Note, the owner is always able to
    /// upload code or instantiate contracts.
    Nobody,
    /// Any account is allowed to perform the action
    Everybody,
    /// Some whitelisted accounts or the owner can perform the action.
    Somebodies(BTreeSet<Addr>),
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(deny_unknown_fields)]
pub struct BlockInfo {
    pub height: u64,
    pub timestamp: Timestamp,
    pub hash: Hash256,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractInfo {
    pub code_hash: Hash256,
    pub label: Option<String>,
    pub admin: Option<Addr>,
}

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
    // `None` means the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub result: GenericResult<()>,
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

#[derive(Debug, PartialEq, Eq)]
pub struct TxSuccess {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub events: TxEvents,
}

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
pub struct MsgsAndBackrunEvents {
    pub msgs: Vec<EventStatus<Event>>,
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

#[derive(Debug)]
/// Outcome of executing a block.
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash256,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<CronOutcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<TxOutcome>,
}
