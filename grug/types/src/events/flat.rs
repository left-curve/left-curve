#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;

#[cfg(feature = "async-graphql")]
use async_graphql::{ComplexObject, Context, Enum, Result, SimpleObject};

use {
    super::FlattenStatus,
    crate::{
        Addr, CheckedContractEvent, Coins, CommitmentStatus, EvtConfigure, EvtUpload, Hash256,
        Json, Label, ReplyOn, Timestamp, TxEvents,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
    strum_macros::{Display, EnumDiscriminants},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEventInfo {
    pub id: EventId,
    pub parent_id: EventId,
    pub commitment_status: FlatCommitmentStatus,
    pub event_status: FlatEventStatus,
    pub event: FlatEvent,
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Display,
    EnumDiscriminants,
)]
#[serde(rename_all = "snake_case")]
pub enum FlatEventStatus {
    Ok,
    Failed(String),
    NestedFailed,
    Handled(String),
}

impl From<&FlatEventStatus> for i16 {
    fn from(status: &FlatEventStatus) -> i16 {
        match status {
            FlatEventStatus::Ok => 0,
            FlatEventStatus::Failed(_) => 1,
            FlatEventStatus::NestedFailed => 2,
            FlatEventStatus::Handled(_) => 3,
        }
    }
}

impl FlatEventStatus {
    pub fn as_i16(&self) -> i16 {
        i16::from(self)
    }
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Display,
)]
#[serde(rename_all = "snake_case")]
pub enum FlatCommitmentStatus {
    Committed,
    Failed,
    Reverted,
}

impl From<&FlatCommitmentStatus> for i16 {
    fn from(status: &FlatCommitmentStatus) -> i16 {
        match status {
            FlatCommitmentStatus::Committed => 0,
            FlatCommitmentStatus::Failed => 1,
            FlatCommitmentStatus::Reverted => 2,
        }
    }
}

impl FlatCommitmentStatus {
    pub fn as_i16(&self) -> i16 {
        i16::from(self)
    }
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Copy,
    Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "sea-orm", derive(EnumIter, DeriveActiveEnum))]
#[cfg_attr(feature = "sea-orm", sea_orm(rs_type = "i32", db_type = "Integer"))]
#[cfg_attr(feature = "async-graphql", derive(Enum))]
pub enum FlatCategory {
    #[strum(serialize = "0")]
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 0))]
    Cron,
    #[strum(serialize = "1")]
    #[cfg_attr(feature = "sea-orm", sea_orm(num_value = 1))]
    Tx,
}

/// Details about a specific Event
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventId {
    /// Block height where the event was emitted.
    pub block: u64,
    /// The category of the event.
    pub category: FlatCategory,
    /// The index within the category, starts with 0.
    pub category_index: u32,
    /// The index of the message if any.
    pub message_index: Option<u32>,
    /// The index of the event within the block, starts with 0.
    pub event_index: u32,
}

impl EventId {
    pub fn new(block: u64, category: FlatCategory, category_index: u32, event_index: u32) -> Self {
        Self {
            block,
            category,
            category_index,
            event_index,
            message_index: None,
        }
    }

    pub fn clone_with_event_index(&self, event_index: u32) -> Self {
        Self {
            block: self.block,
            category: self.category,
            category_index: self.category_index,
            event_index,
            message_index: self.message_index,
        }
    }

    pub fn increment_idx(&mut self, items: &[FlatEventInfo]) {
        if let Some(item) = items.last() {
            self.event_index = item.id.event_index + 1;
        }
    }
}

// ------------------------------ Flat Events -------------------------------

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FlatEvent {
    Configure(EvtConfigure),
    /// Coins were transferred from one account to another.
    Transfer(FlatEvtTransfer),
    /// A wasm binary code was uploaded.
    Upload(EvtUpload),
    /// A new contract was instantiated.
    Instantiate(FlatEvtInstantiate),
    /// A contract was executed.
    Execute(FlatEvtExecute),
    /// A contract was migrated to a new code hash.
    Migrate(FlatEvtMigrate),
    /// A contract was replied the outcome of its submessage.
    Reply(FlatEvtReply),
    /// A contract authenticated a transaction.
    Authenticate(FlatEvtAuthenticate),
    /// A contract backran a transaction.
    Backrun(FlatEvtBackrun),
    /// The taxman withheld the fee for a transaction.
    Withhold(FlatEvtWithhold),
    /// The taxman finalized the fee for a transaction.
    Finalize(FlatEvtFinalize),
    /// A cronjob was executed.
    Cron(FlatEvtCron),
    /// A guest was called.
    Guest(FlatEvtGuest),
    /// A contract event was emitted.
    ContractEvent(CheckedContractEvent),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtTransfer {
    pub sender: Addr,
    pub transfers: BTreeMap<Addr, Coins>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtInstantiate {
    pub sender: Addr,
    pub contract: Addr,
    pub code_hash: Hash256,
    pub label: Option<Label>,
    pub admin: Option<Addr>,
    pub instantiate_msg: Json,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtExecute {
    pub sender: Addr,
    pub contract: Addr,
    pub funds: Coins,
    pub execute_msg: Json,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtMigrate {
    pub sender: Addr,
    pub contract: Addr,
    pub migrate_msg: Json,
    pub old_code_hash: Option<Hash256>,
    pub new_code_hash: Hash256,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtReply {
    pub contract: Addr,
    pub reply_on: ReplyOn,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtAuthenticate {
    pub sender: Addr,
    pub backrun: bool,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtBackrun {
    pub sender: Addr,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtWithhold {
    pub sender: Addr,
    pub gas_limit: u64,
    pub taxman: Option<Addr>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtFinalize {
    pub sender: Addr,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub taxman: Option<Addr>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtCron {
    pub contract: Addr,
    /// The timestamp of this cronjob execution.
    pub time: Timestamp,
    /// The timestamp of the next cronjob execution is scheduled.
    pub next: Timestamp,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub method: String,
}

pub fn flatten_commitment_status<T>(
    next_id: &mut EventId,
    commitment: CommitmentStatus<T>,
) -> Vec<FlatEventInfo>
where
    T: FlattenStatus,
{
    // To void the need of `parent: Option<EventId>` we use event_index set to 0
    let parent_id = next_id.clone_with_event_index(0);

    match commitment {
        CommitmentStatus::Committed(event) => {
            event.flatten_status(&parent_id, next_id, FlatCommitmentStatus::Committed)
        },
        CommitmentStatus::Failed { event, .. } => {
            event.flatten_status(&parent_id, next_id, FlatCommitmentStatus::Failed)
        },
        CommitmentStatus::Reverted { event, .. } => {
            event.flatten_status(&parent_id, next_id, FlatCommitmentStatus::Reverted)
        },
        CommitmentStatus::NotReached => vec![],
    }
}

pub fn flatten_tx_events(tx_events: TxEvents, block_id: u64, tx_id: u32) -> Vec<FlatEventInfo> {
    let mut flat_events = vec![];

    let mut next_id = EventId::new(block_id, FlatCategory::Tx, tx_id, 0);

    let events = flatten_commitment_status(&mut next_id, tx_events.withhold);
    flat_events.extend(events);

    let events = flatten_commitment_status(&mut next_id, tx_events.authenticate);
    flat_events.extend(events);

    let events = flatten_commitment_status(&mut next_id, tx_events.msgs_and_backrun);
    flat_events.extend(events);

    let events = flatten_commitment_status(&mut next_id, tx_events.finalize);
    flat_events.extend(events);

    flat_events
}
