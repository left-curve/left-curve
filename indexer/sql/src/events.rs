use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{
        Addr, Coins, CommitmentStatus, ContractEvent, Event, EventStatus, EvtAuthenticate,
        EvtBackrun, EvtConfigure, EvtCron, EvtExecute, EvtFinalize, EvtGuest, EvtInstantiate,
        EvtMigrate, EvtReply, EvtTransfer, EvtUpload, EvtWithhold, Hash256, Json, Label,
        MsgsAndBackrunEvents, ReplyOn, SubEvent, SubEventStatus, Timestamp, TxEvents,
    },
    serde::{de::Visitor, Deserialize, Serialize},
    std::{
        fmt::{self, Display},
        str::FromStr,
    },
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct IndexEvent {
    pub id: EventId,
    pub parent_id: EventId,
    pub commitment_status: IndexCommitmentStatus,
    pub event_status: IndexEventStatus,
    pub event: FlattenEvent,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexEventStatus {
    Ok,
    Failed(String),
    SubFailed,
    Handled(String),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexCommitmentStatus {
    Committed,
    Failed,
    Reverted,
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum IndexCategory {
    Tx,
    Cron,
}

impl Display for IndexCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IndexCategory::Tx => write!(f, "tx"),
            IndexCategory::Cron => write!(f, "cron"),
        }
    }
}

impl FromStr for IndexCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tx" => Ok(IndexCategory::Tx),
            "cron" => Ok(IndexCategory::Cron),
            _ => Err(format!("Invalid category: {}", s)),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventId {
    pub block: u64,
    pub category: IndexCategory,
    pub category_index: u32,
    pub event_index: u32,
}

impl EventId {
    pub fn new(block: u64, category: IndexCategory, category_index: u32) -> Self {
        Self {
            block,
            category,
            category_index,
            event_index: 0,
        }
    }

    pub fn clone_with_event_index(&self, event_index: u32) -> Self {
        Self {
            block: self.block,
            category: self.category,
            category_index: self.category_index,
            event_index,
        }
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}",
            self.block, self.category, self.category_index, self.event_index
        )
    }
}

impl serde::Serialize for EventId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for EventId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(EventVisitor)
    }
}

struct EventVisitor;

impl Visitor<'_> for EventVisitor {
    type Value = EventId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format 'block-tx_index-event_index'")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let parts: Vec<&str> = value.split('-').collect();

        if parts.len() != 4 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(value),
                &self,
            ));
        }

        let block = parts[0].parse().map_err(serde::de::Error::custom)?;

        let category = parts[1].parse().map_err(serde::de::Error::custom)?;

        let tx_index = parts[2].parse().map_err(serde::de::Error::custom)?;
        let event_index = parts[3].parse().map_err(serde::de::Error::custom)?;

        Ok(EventId {
            block,
            category,
            category_index: tx_index,
            event_index,
        })
    }
}

pub fn get_next_event_index(events: &[IndexEvent]) -> Option<u32> {
    events.last().map(|event| event.id.event_index + 1)
}

// ------------------------------ Flatten Events -------------------------------

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlattenEvent {
    Configure(EvtConfigure),
    /// Coins were transferred from one account to another.
    Transfer(FlattenEvtTransfer),
    /// A wasm binary code was uploaded.
    Upload(EvtUpload),
    /// A new contract was instantiated.
    Instantiate(FlattenEvtInstantiate),
    /// A contract was executed.
    Execute(FlattenEvtExecute),
    /// A contract was migrated to a new code hash.
    Migrate(FlattenEvtMigrate),
    /// A contract was replied the outcome of its submessage.
    Reply(FlattenEvtReply),
    /// A contract authenticated a transaction.
    Authenticate(FlattenEvtAuthenticate),
    /// A contract backran a transaction.
    Backrun(FlattenEvtBackrun),
    /// The taxman withheld the fee for a transaction.
    Withhold(FlattenEvtWithhold),
    /// The taxman finalized the fee for a transaction.
    Finalize(FlattenEvtFinalize),
    /// A cronjob was executed.
    Cron(FlattenEvtCron),
    Guest(FlattenEvtGuest),
    ContractEvent(ContractEvent),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtTransfer {
    pub sender: Addr,
    pub recipient: Addr,
    pub coins: Coins,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtInstantiate {
    pub sender: Addr,
    pub contract: Addr,
    pub code_hash: Hash256,
    pub label: Option<Label>,
    pub admin: Option<Addr>,
    pub instantiate_msg: Json,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtExecute {
    pub sender: Addr,
    pub contract: Addr,
    pub funds: Coins,
    pub execute_msg: Json,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtMigrate {
    pub sender: Addr,
    pub contract: Addr,
    pub migrate_msg: Json,
    pub old_code_hash: Option<Hash256>,
    pub new_code_hash: Hash256,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtReply {
    pub contract: Addr,
    pub reply_on: ReplyOn,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtAuthenticate {
    pub sender: Addr,
    pub backrun: bool,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtBackrun {
    pub sender: Addr,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtWithhold {
    pub sender: Addr,
    pub gas_limit: u64,
    pub taxman: Option<Addr>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtFinalize {
    pub sender: Addr,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub taxman: Option<Addr>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtCron {
    pub contract: Addr,
    /// The timestamp of this cronjob execution.
    pub time: Timestamp,
    /// The timestamp of the next cronjob execution is scheduled.
    pub next: Timestamp,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlattenEvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub method: String,
}

// ------------------------------- Trait Flatten -------------------------------

pub trait Flatten {
    fn flat(
        self,
        parent: &EventId,
        next: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent>;
}

pub trait FlattenStatus {
    fn flat_status(
        self,
        parent: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
    ) -> Vec<IndexEvent>;
}

fn flat_commitment_status<T>(
    block_id: u64,
    category: IndexCategory,
    category_id: u32,
    mut next_id: u32,
    commitment: CommitmentStatus<T>,
) -> (Vec<IndexEvent>, u32)
where
    T: FlattenStatus,
{
    let parent = EventId {
        block: block_id,
        category,
        category_index: category_id,
        event_index: 0,
    };

    let events = match commitment {
        CommitmentStatus::Committed(event) => {
            event.flat_status(&parent, next_id, IndexCommitmentStatus::Committed)
        },
        CommitmentStatus::Failed { event, .. } => {
            event.flat_status(&parent, next_id, IndexCommitmentStatus::Failed)
        },
        CommitmentStatus::Reverted { event, .. } => {
            event.flat_status(&parent, next_id, IndexCommitmentStatus::Reverted)
        },
        CommitmentStatus::NotReached => vec![],
    };

    next_id = get_next_event_index(&events).unwrap_or(next_id);

    (events, next_id)
}

pub fn flat_tx_events(tx_events: TxEvents, block_id: u64, tx_id: u32) -> Vec<IndexEvent> {
    let mut flat_events = vec![];

    let (events, next_id) =
        flat_commitment_status(block_id, IndexCategory::Tx, tx_id, 1, tx_events.withhold);
    flat_events.extend(events);
    let (events, next_id) = flat_commitment_status(
        block_id,
        IndexCategory::Tx,
        tx_id,
        next_id,
        tx_events.authenticate,
    );
    flat_events.extend(events);
    let (events, next_id) = flat_commitment_status(
        block_id,
        IndexCategory::Tx,
        tx_id,
        next_id,
        tx_events.msgs_and_backrun,
    );
    flat_events.extend(events);
    let (events, _) = flat_commitment_status(
        block_id,
        IndexCategory::Tx,
        tx_id,
        next_id,
        tx_events.finalize,
    );
    flat_events.extend(events);

    flat_events
}

pub fn flat_cron(
    cron: CommitmentStatus<EventStatus<EvtCron>>,
    block_id: u64,
    cron_id: u32,
) -> Vec<IndexEvent> {
    let (events, _) = flat_commitment_status(block_id, IndexCategory::Cron, cron_id, 1, cron);

    events
}

// -------------------------- impl Flatten for Status --------------------------

impl<T> FlattenStatus for EventStatus<T>
where
    T: Flatten,
{
    fn flat_status(
        self,
        parent: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
    ) -> Vec<IndexEvent> {
        match self {
            EventStatus::Ok(event) => event.flat(parent, next_id, commitment, IndexEventStatus::Ok),
            EventStatus::Failed { event, error } => {
                event.flat(parent, next_id, commitment, IndexEventStatus::Failed(error))
            },
            EventStatus::NestedFailed(event) => {
                event.flat(parent, next_id, commitment, IndexEventStatus::SubFailed)
            },
            EventStatus::NotReached => vec![],
        }
    }
}

impl FlattenStatus for SubEventStatus {
    fn flat_status(
        self,
        parent: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
    ) -> Vec<IndexEvent> {
        let (event, commitment, status) = match self {
            SubEventStatus::Ok(event) => (event, commitment, IndexEventStatus::Ok),
            SubEventStatus::NestedFailed(event) => (event, commitment, IndexEventStatus::SubFailed),
            SubEventStatus::Failed { event, error } => {
                (event, commitment, IndexEventStatus::Failed(error))
            },
            // SubEventStatus::Handled is a particular case.
            // It means that a submsg fails but the error has been handled on reply.
            // In this case, the commitment status is Failed regardless of the original commitment status.
            SubEventStatus::Handled { event, error } => (
                event,
                IndexCommitmentStatus::Failed,
                IndexEventStatus::Handled(error),
            ),
        };

        event.flat(parent, next_id, commitment, status)
    }
}

impl FlattenStatus for MsgsAndBackrunEvents {
    fn flat_status(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
    ) -> Vec<IndexEvent> {
        let mut events = vec![];

        for msgs in self.msgs {
            let i_events = msgs.flat_status(parent_id, next_id, commitment.clone());
            // +1 is not needed here because the next_id is already incremented
            next_id = get_next_event_index(&i_events).unwrap_or(next_id);
            events.extend(i_events);
        }

        events.extend(self.backrun.flat_status(parent_id, next_id, commitment));

        events
    }
}

// -------------------------- impl Flatten for Events --------------------------

impl Flatten for EvtConfigure {
    fn flat(
        self,
        parent: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        vec![IndexEvent {
            id: parent.clone_with_event_index(next_id),
            parent_id: parent.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlattenEvent::Configure(self),
        }]
    }
}

impl Flatten for EvtTransfer {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Transfer(FlattenEvtTransfer {
                sender: self.sender,
                recipient: self.recipient,
                coins: self.coins,
            }),
        }];

        next_id += 1;

        let bank_guest = self
            .bank_guest
            .flat_status(&current_id, next_id, commitment.clone());

        next_id = get_next_event_index(&bank_guest).unwrap_or(next_id);

        events.extend(bank_guest);

        let receive_guest = self
            .receive_guest
            .flat_status(&current_id, next_id, commitment);

        events.extend(receive_guest);

        events
    }
}

impl Flatten for EvtUpload {
    fn flat(
        self,
        parent_id: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        vec![IndexEvent {
            id: parent_id.clone_with_event_index(next_id),
            parent_id: parent_id.clone(),
            commitment_status: commitment,
            event_status: status,
            event: FlattenEvent::Upload(self),
        }]
    }
}

impl Flatten for EvtInstantiate {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Instantiate(FlattenEvtInstantiate {
                sender: self.sender,
                contract: self.contract,
                code_hash: self.code_hash,
                label: self.label,
                admin: self.admin,
                instantiate_msg: self.instantiate_msg,
            }),
        }];

        next_id += 1;

        let transfer = self
            .transfer_event
            .flat_status(&current_id, next_id, commitment.clone());

        next_id = get_next_event_index(&transfer).unwrap_or(next_id);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtExecute {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Execute(FlattenEvtExecute {
                sender: self.sender,
                contract: self.contract,
                funds: self.funds,
                execute_msg: self.execute_msg,
            }),
        }];

        next_id += 1;

        let transfer = self
            .transfer_event
            .flat_status(&current_id, next_id, commitment.clone());

        next_id = get_next_event_index(&transfer).unwrap_or(next_id);

        events.extend(transfer);

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtMigrate {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Migrate(FlattenEvtMigrate {
                sender: self.sender,
                contract: self.contract,
                migrate_msg: self.migrate_msg,
                old_code_hash: self.old_code_hash,
                new_code_hash: self.new_code_hash,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtBackrun {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Backrun(FlattenEvtBackrun {
                sender: self.sender,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtWithhold {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Withhold(FlattenEvtWithhold {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtFinalize {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Finalize(FlattenEvtFinalize {
                sender: self.sender,
                gas_limit: self.gas_limit,
                taxman: self.taxman,
                gas_used: self.gas_used,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtCron {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Cron(FlattenEvtCron {
                contract: self.contract,
                time: self.time,
                next: self.next,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for Event {
    fn flat(
        self,
        parent_id: &EventId,
        next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        match self {
            Event::Configure(evt_configure) => {
                evt_configure.flat(parent_id, next_id, commitment, status)
            },
            Event::Transfer(evt_transfer) => {
                evt_transfer.flat(parent_id, next_id, commitment, status)
            },
            Event::Upload(evt_upload) => evt_upload.flat(parent_id, next_id, commitment, status),
            Event::Instantiate(evt_instantiate) => {
                evt_instantiate.flat(parent_id, next_id, commitment, status)
            },
            Event::Execute(evt_execute) => evt_execute.flat(parent_id, next_id, commitment, status),
            Event::Migrate(evt_migrate) => evt_migrate.flat(parent_id, next_id, commitment, status),
            Event::Reply(evt_reply) => evt_reply.flat(parent_id, next_id, commitment, status),
            Event::Authenticate(evt_authenticate) => {
                evt_authenticate.flat(parent_id, next_id, commitment, status)
            },
            Event::Backrun(evt_backrun) => evt_backrun.flat(parent_id, next_id, commitment, status),
            Event::Withhold(evt_withhold) => {
                evt_withhold.flat(parent_id, next_id, commitment, status)
            },
            Event::Finalize(evt_finalize) => {
                evt_finalize.flat(parent_id, next_id, commitment, status)
            },
            Event::Cron(evt_cron) => evt_cron.flat(parent_id, next_id, commitment, status),
        }
    }
}

impl Flatten for EvtAuthenticate {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Authenticate(FlattenEvtAuthenticate {
                sender: self.sender,
                backrun: self.backrun,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}

impl Flatten for EvtGuest {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status.clone(),
            event: FlattenEvent::Guest(FlattenEvtGuest {
                contract: self.contract,
                method: self.method,
            }),
        }];

        for contract_event in self.contract_events {
            next_id += 1;

            events.push(IndexEvent {
                id: parent_id.clone_with_event_index(next_id),
                parent_id: parent_id.clone(),
                commitment_status: commitment.clone(),
                event_status: status.clone(),
                event: FlattenEvent::ContractEvent(contract_event),
            });
        }

        for sub_event in self.sub_events {
            next_id += 1;

            let sub_events = sub_event.flat_status(&current_id, next_id, commitment.clone());

            // -1 is needed here because the next_id is already incremented
            next_id = get_next_event_index(&sub_events).unwrap_or(next_id - 1);
            events.extend(sub_events);
        }

        events
    }
}

impl Flatten for SubEvent {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        _status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let mut events = vec![];

        let sub_event = self
            .event
            .flat_status(parent_id, next_id, commitment.clone());

        let reply_events = if let Some(reply) = self.reply {
            next_id = get_next_event_index(&sub_event).unwrap_or(next_id);

            reply.flat_status(parent_id, next_id, commitment)
        } else {
            vec![]
        };

        events.extend(sub_event);
        events.extend(reply_events);

        events
    }
}

impl Flatten for EvtReply {
    fn flat(
        self,
        parent_id: &EventId,
        mut next_id: u32,
        commitment: IndexCommitmentStatus,
        status: IndexEventStatus,
    ) -> Vec<IndexEvent> {
        let current_id = parent_id.clone_with_event_index(next_id);

        let mut events = vec![IndexEvent {
            id: current_id.clone(),
            parent_id: parent_id.clone(),
            commitment_status: commitment.clone(),
            event_status: status,
            event: FlattenEvent::Reply(FlattenEvtReply {
                contract: self.contract,
                reply_on: self.reply_on,
            }),
        }];

        next_id += 1;

        let guest = self
            .guest_event
            .flat_status(&current_id, next_id, commitment);

        events.extend(guest);
        events
    }
}
