use {
    crate::{
        Addr, Coins, CommitmentStatus, ContractEvent, Event, EventStatus, EvtAuthenticate,
        EvtBackrun, EvtConfigure, EvtCron, EvtExecute, EvtFinalize, EvtGuest, EvtInstantiate,
        EvtMigrate, EvtReply, EvtTransfer, EvtUpload, EvtWithhold, Hash256, Json, Label,
        MsgsAndBackrunEvents, ReplyOn, SubEvent, SubEventStatus, Timestamp, TxEvents,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    strum_macros::Display,
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
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FlatEventStatus {
    #[strum(serialize = "0")]
    Ok,
    #[strum(serialize = "1")]
    Failed(String),
    #[strum(serialize = "2")]
    NestedFailed,
    #[strum(serialize = "3")]
    Handled(String),
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq, Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FlatCommitmentStatus {
    #[strum(serialize = "0")]
    Committed,
    #[strum(serialize = "1")]
    Failed,
    #[strum(serialize = "2")]
    Reverted,
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FlatCategory {
    #[strum(serialize = "0")]
    Cron,
    #[strum(serialize = "1")]
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

pub fn get_next_event_index(events: &[FlatEventInfo]) -> Option<u32> {
    events.last().map(|event| event.id.event_index + 1)
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
    Guest(FlatEvtGuest),
    ContractEvent(ContractEvent),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlatEvtTransfer {
    pub sender: Addr,
    pub recipient: Addr,
    pub coins: Coins,
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

// ------------------------------- Trait Flat -------------------------------

pub trait Flatten {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo>;
}

pub trait FlattenStatus {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo>;
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

// -------------------------- impl Flat for Status --------------------------

impl<T> FlattenStatus for EventStatus<T>
where
    T: Flatten,
{
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        match self {
            EventStatus::Ok(event) => {
                event.flatten(parent_id, next_id, commitment, FlatEventStatus::Ok)
            },
            EventStatus::Failed { event, error } => event.flatten(
                parent_id,
                next_id,
                commitment,
                FlatEventStatus::Failed(error),
            ),
            EventStatus::NestedFailed(event) => event.flatten(
                parent_id,
                next_id,
                commitment,
                FlatEventStatus::NestedFailed,
            ),
            EventStatus::NotReached => vec![],
        }
    }
}

impl FlattenStatus for SubEventStatus {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        let (event, commitment, status) = match self {
            SubEventStatus::Ok(event) => (event, commitment, FlatEventStatus::Ok),
            SubEventStatus::NestedFailed(event) => {
                (event, commitment, FlatEventStatus::NestedFailed)
            },
            SubEventStatus::Failed { event, error } => {
                (event, commitment, FlatEventStatus::Failed(error))
            },
            // SubEventStatus::Handled is a particular case.
            // It means that a submsg fails but the error has been handled on reply.
            // In this case, the commitment status is Failed regardless of the original commitment status.
            SubEventStatus::Handled { event, error } => (
                event,
                FlatCommitmentStatus::Failed,
                FlatEventStatus::Handled(error),
            ),
        };

        event.flatten(parent_id, next_id, commitment, status)
    }
}

impl FlattenStatus for MsgsAndBackrunEvents {
    fn flatten_status(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![];

        for (msg_idx, msgs) in self.msgs.into_iter().enumerate() {
            next_id.message_index = Some(msg_idx as u32);

            let i_events = msgs.flatten_status(parent_id, next_id, commitment.clone());

            next_id.increment_idx(&i_events);
            events.extend(i_events);
        }
        next_id.message_index = None;

        events.extend(self.backrun.flatten_status(parent_id, next_id, commitment));

        events
    }
}

// -------------------------- impl Flat for Events --------------------------

impl Flatten for EvtConfigure {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Configure(self),
        }]
    }
}

impl Flatten for EvtTransfer {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Transfer(FlatEvtTransfer {
                sender: self.sender,
                recipient: self.recipient,
                coins: self.coins,
            }),
        }];

        next_id.event_index += 1;

        let bank_guest = self
            .bank_guest
            .flatten_status(parent_id, next_id, commitment.clone());

        next_id.increment_idx(&bank_guest);

        events.extend(bank_guest);

        let receive_guest = self
            .receive_guest
            .flatten_status(parent_id, next_id, commitment);

        events.extend(receive_guest);

        events
    }
}

impl Flatten for EvtUpload {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        vec![FlatEventInfo {
            id: parent_id.clone_with_event_index(next_id.event_index),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlatEvent::Upload(self),
        }]
    }
}

impl Flatten for EvtInstantiate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Instantiate(FlatEvtInstantiate {
                sender: self.sender,
                contract: self.contract,
                code_hash: self.code_hash,
                label: self.label,
                admin: self.admin,
                instantiate_msg: self.instantiate_msg,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let transfer = self
            .transfer_event
            .flatten_status(&parent_id, next_id, commitment.clone());

        next_id.increment_idx(&transfer);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtExecute {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Execute(FlatEvtExecute {
                sender: self.sender,
                contract: self.contract,
                funds: self.funds,
                execute_msg: self.execute_msg,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let transfer = self
            .transfer_event
            .flatten_status(&parent_id, next_id, commitment.clone());

        next_id.increment_idx(&transfer);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtMigrate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Migrate(FlatEvtMigrate {
                sender: self.sender,
                contract: self.contract,
                migrate_msg: self.migrate_msg,
                old_code_hash: self.old_code_hash,
                new_code_hash: self.new_code_hash,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtBackrun {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Backrun(FlatEvtBackrun {
                sender: self.sender,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtWithhold {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Withhold(FlatEvtWithhold {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtFinalize {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Finalize(FlatEvtFinalize {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
                gas_used: self.gas_used,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtCron {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Cron(FlatEvtCron {
                contract: self.contract,
                time: self.time,
                next: self.next,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for Event {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        match self {
            Event::Configure(evt_configure) => {
                evt_configure.flatten(parent_id, next_id, commitment, status)
            },
            Event::Transfer(evt_transfer) => {
                evt_transfer.flatten(parent_id, next_id, commitment, status)
            },
            Event::Upload(evt_upload) => evt_upload.flatten(parent_id, next_id, commitment, status),
            Event::Instantiate(evt_instantiate) => {
                evt_instantiate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Execute(evt_execute) => {
                evt_execute.flatten(parent_id, next_id, commitment, status)
            },
            Event::Migrate(evt_migrate) => {
                evt_migrate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Reply(evt_reply) => evt_reply.flatten(parent_id, next_id, commitment, status),
            Event::Authenticate(evt_authenticate) => {
                evt_authenticate.flatten(parent_id, next_id, commitment, status)
            },
            Event::Backrun(evt_backrun) => {
                evt_backrun.flatten(parent_id, next_id, commitment, status)
            },
            Event::Withhold(evt_withhold) => {
                evt_withhold.flatten(parent_id, next_id, commitment, status)
            },
            Event::Finalize(evt_finalize) => {
                evt_finalize.flatten(parent_id, next_id, commitment, status)
            },
            Event::Cron(evt_cron) => evt_cron.flatten(parent_id, next_id, commitment, status),
        }
    }
}

impl Flatten for EvtAuthenticate {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Authenticate(FlatEvtAuthenticate {
                sender: self.sender,
                backrun: self.backrun,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtGuest {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let current_id = parent_id.clone_with_event_index(next_id.event_index);

        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status.clone(),
            event: FlatEvent::Guest(FlatEvtGuest {
                contract: self.contract,
                method: self.method,
            }),
        }];

        next_id.event_index += 1;

        for contract_event in self.contract_events {
            events.push(FlatEventInfo {
                id: next_id.clone(),
                parent_id: parent_id.clone(),
                commitment_status: commitment.clone(),
                event_status: status.clone(),
                event: FlatEvent::ContractEvent(contract_event),
            });

            next_id.event_index += 1;
        }

        for sub_event in self.sub_events {
            let sub_events = sub_event.flatten_status(&current_id, next_id, commitment.clone());

            next_id.increment_idx(&sub_events);
            // -1 is needed here because the next_id is already incremented
            // next_id.event_index =
            //     get_next_event_index(&sub_events).unwrap_or(next_id.event_index - 1);
            events.extend(sub_events);
        }

        events
    }
}

impl Flatten for SubEvent {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        _status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![];

        let sub_event = self
            .event
            .flatten_status(parent_id, next_id, commitment.clone());

        let reply_events = if let Some(reply) = self.reply {
            next_id.increment_idx(&sub_event);
            reply.flatten_status(parent_id, next_id, commitment)
        } else {
            vec![]
        };

        events.extend(sub_event);
        events.extend(reply_events);

        events
    }
}

impl Flatten for EvtReply {
    fn flatten(
        self,
        parent_id: &EventId,
        next_id: &mut EventId,
        commitment: FlatCommitmentStatus,
        status: FlatEventStatus,
    ) -> Vec<FlatEventInfo> {
        let mut events = vec![FlatEventInfo {
            id: next_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlatEvent::Reply(FlatEvtReply {
                contract: self.contract,
                reply_on: self.reply_on,
            }),
        }];

        let parent_id = next_id.clone();
        next_id.event_index += 1;

        let guest = self
            .guest_event
            .flatten_status(&parent_id, next_id, commitment);

        events.extend(guest);
        events
    }
}
