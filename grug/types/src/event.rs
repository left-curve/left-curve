use {
    crate::{
        Addr, Coins, ContractEvent, EventStatus, HandleEventStatus, Hash256, Json, Label, ReplyOn,
        Timestamp,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// The chain- or app-level configurations were updated.
    Configure(EvtConfigure),
    /// Coins were transferred from one account to another.
    Transfer(EvtTransfer),
    /// A wasm binary code was uploaded.
    Upload(EvtUpload),
    /// A new contract was instantiated.
    Instantiate(EvtInstantiate),
    /// A contract was executed.
    Execute(EvtExecute),
    /// A contract was migrated to a new code hash.
    Migrate(EvtMigrate),
    /// A contract was replied the outcome of its submessage.
    Reply(EvtReply),
    /// A contract authenticated a transaction.
    Authenticate(EvtAuthenticate),
    /// A contract backran a transaction.
    Backrun(EvtBackrun),
    /// The taxman withheld the fee for a transaction.
    Withhold(EvtWithhold),
    /// The taxman finalized the fee for a transaction.
    Finalize(EvtFinalize),
    /// A cronjob was executed.
    Cron(EvtCron),
    // TODO: IBC events
}

impl Event {
    pub fn reply(contract: Addr, reply_on: ReplyOn, guest_event: EventStatus<EvtGuest>) -> Self {
        Self::Reply(EvtReply {
            contract,
            reply_on,
            guest_event,
        })
    }

    pub fn cron(
        contract: Addr,
        time: Timestamp,
        next: Timestamp,
        guest_event: EventStatus<EvtGuest>,
    ) -> Self {
        Self::Cron(EvtCron {
            contract,
            time,
            next,
            guest_event,
        })
    }
}

/// An event indicating that the chain- or app-level configurations were updated.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtConfigure {
    pub sender: Addr,
    // TODO: not sure what else we need here. the old and new configs?
}

/// An event indicating that coins were transferred from one account to another.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtTransfer {
    pub sender: Addr,
    pub recipient: Addr,
    pub coins: Coins,
    pub bank_guest: EventStatus<EvtGuest>,
    pub receive_guest: EventStatus<EvtGuest>,
}

impl EvtTransfer {
    pub fn base(sender: Addr, recipient: Addr, coins: Coins) -> Self {
        Self {
            sender,
            recipient,
            coins,
            bank_guest: EventStatus::NotReached,
            receive_guest: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a wasm binary code was uploaded.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtUpload {
    pub sender: Addr,
    pub code_hash: Hash256,
}

/// An event indicating that a new contract was instantiated.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtInstantiate {
    pub sender: Addr,
    pub contract: Addr,
    pub code_hash: Hash256,
    pub label: Option<Label>,
    pub admin: Option<Addr>,
    pub instantiate_msg: Json,
    pub transfer_event: EventStatus<EvtTransfer>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtInstantiate {
    pub fn base(sender: Addr, code_hash: Hash256, contract: Addr, instantiate_msg: Json) -> Self {
        Self {
            sender,
            contract,
            code_hash,
            instantiate_msg,
            label: None,
            admin: None,
            transfer_event: EventStatus::NotReached,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtExecute {
    pub sender: Addr,
    pub contract: Addr,
    pub funds: Coins,
    pub execute_msg: Json,
    pub transfer_event: EventStatus<EvtTransfer>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtExecute {
    pub fn base(sender: Addr, contract: Addr, funds: Coins, execute_msg: Json) -> Self {
        Self {
            sender,
            contract,
            funds,
            execute_msg,
            transfer_event: EventStatus::NotReached,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract was migrated to a new code hash.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtMigrate {
    pub sender: Addr,
    pub contract: Addr,
    pub migrate_msg: Json,
    pub old_code_hash: Option<Hash256>,
    pub new_code_hash: Hash256,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtMigrate {
    pub fn base(sender: Addr, contract: Addr, migrate_msg: Json, new_code_hash: Hash256) -> Self {
        Self {
            sender,
            contract,
            migrate_msg,
            old_code_hash: None,
            new_code_hash,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract was replied the outcome of its submessage.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtReply {
    pub contract: Addr,
    pub reply_on: ReplyOn,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtReply {
    pub fn base(contract: Addr, reply_on: ReplyOn) -> Self {
        Self {
            contract,
            reply_on,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract authenticated a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtAuthenticate {
    pub sender: Addr,
    pub backrun: bool,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtAuthenticate {
    pub fn base(sender: Addr) -> Self {
        Self {
            sender,
            backrun: false,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract backran a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtBackrun {
    pub sender: Addr,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtBackrun {
    pub fn base(sender: Addr) -> Self {
        Self {
            sender,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that The taxman withheld the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtWithhold {
    pub sender: Addr,
    pub gas_limit: u64,
    pub taxman: Option<Addr>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtWithhold {
    pub fn base(sender: Addr, gas_limit: u64) -> Self {
        Self {
            sender,
            gas_limit,
            taxman: None,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that the taxman finalized the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtFinalize {
    pub sender: Addr,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub taxman: Option<Addr>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtFinalize {
    pub fn base(sender: Addr, gas_limit: u64, gas_used: u64) -> Self {
        Self {
            sender,
            gas_limit,
            gas_used,
            taxman: None,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a cronjob was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtCron {
    pub contract: Addr,
    /// The timestamp of this cronjob execution.
    pub time: Timestamp,
    /// The timestamp of the next cronjob execution is scheduled.
    pub next: Timestamp,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtCron {
    pub fn base(contract: Addr, time: Timestamp, next: Timestamp) -> Self {
        Self {
            contract,
            time,
            next,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract emitted a custom event.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub method: String,
    /// Sub events emitted by the contract.
    pub contract_events: Vec<ContractEvent>,
    /// All events emitted by a submessage.
    pub sub_events: Vec<EventStatus<SubEvent>>,
}

impl EvtGuest {
    pub fn base(contract: Addr, method: &'static str) -> Self {
        Self {
            contract,
            method: method.to_string(),
            contract_events: Vec::new(),
            sub_events: Vec::new(),
        }
    }
}

/// An event generated by a submessage.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct SubEvent {
    /// Event generated by a submessage.
    pub event: HandleEventStatus,
    /// None means the contract did not request a reply.
    pub reply: Option<EventStatus<EvtReply>>,
}
