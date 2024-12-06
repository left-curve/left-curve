use {
    crate::{Addr, Coins, ContractEvent, Hash256, Label, ReplyOn, ReplyOnDiscriminants, Timestamp},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    strum_macros::AsRefStr,
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
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
            reply_on: ReplyOnDiscriminants::from(reply_on),
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

    /// Shortcut to get the name of the variant.
    pub fn variant_name(&self) -> &str {
        self.as_ref()
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
    pub transfer_event: EventStatus<EvtTransfer>,
    pub guest_event: EventStatus<EvtGuest>,
    // TODO: is it necessary to include the InstantiateMsg?
}

impl EvtInstantiate {
    pub fn base(sender: Addr, code_hash: Hash256, contract: Addr) -> Self {
        Self {
            sender,
            contract,
            code_hash,
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
    pub transfer_event: EventStatus<EvtTransfer>,
    pub guest_event: EventStatus<EvtGuest>,
    // TODO: is it necessary to include the ExecuteMsg?
}

impl EvtExecute {
    pub fn base(sender: Addr, contract: Addr, funds: Coins) -> Self {
        Self {
            sender,
            contract,
            funds,
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
    pub old_code_hash: Option<Hash256>,
    pub new_code_hash: Hash256,
    pub guest_event: EventStatus<EvtGuest>,
    // TODO: is it necessary to include the MigrateMsg?
}

impl EvtMigrate {
    pub fn base(sender: Addr, contract: Addr, new_code_hash: Hash256) -> Self {
        Self {
            sender,
            contract,
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
    pub reply_on: ReplyOnDiscriminants,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtReply {
    pub fn base(contract: Addr, reply_on: &ReplyOn) -> Self {
        Self {
            contract,
            reply_on: ReplyOnDiscriminants::from(reply_on),
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that a contract authenticated a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtAuthenticate {
    pub sender: Addr,
    pub backrun: Option<bool>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtAuthenticate {
    pub fn base(sender: Addr) -> Self {
        Self {
            sender,
            backrun: None,
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
    pub taxman: Option<Addr>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtWithhold {
    pub fn base(sender: Addr) -> Self {
        Self {
            sender,
            taxman: None,
            guest_event: EventStatus::NotReached,
        }
    }
}

/// An event indicating that the taxman finalized the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtFinalize {
    pub sender: Addr,
    pub taxman: Option<Addr>,
    pub guest_event: EventStatus<EvtGuest>,
}

impl EvtFinalize {
    pub fn base(sender: Addr) -> Self {
        Self {
            sender,
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

/// An event indicating that a contract emitted a custom event.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub method: String,
    /// Sub events emitted by the contract.
    pub contract_events: Vec<ContractEvent>,
    /// All events emitted by a submessage.
    pub sub_events: Vec<SubEventStatus>,
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

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxEvents {
    pub withhold: EventStatus<EvtAuthenticate>,
    pub authenticate: EventStatus<EvtAuthenticate>,
    pub messaged: EventStatus<Vec<Event>>,
    pub backrun: EventStatus<EvtBackrun>,
    pub finalize: EventStatus<EvtFinalize>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
/// An event generated by a submessage.
pub struct SubEvent {
    /// Event generated by a submessage.
    pub event: HandleEventStatus,
    /// None means the contract did not request a reply.
    pub reply: Option<EventStatus<EvtReply>>,
}

//  ------------------------------ Event Statuses ------------------------------

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus<T> {
    /// The event succeeded.
    /// State changes are committed.
    Ok(T),
    /// The event failed.
    /// State changes are reverted.
    Failed { event: T, error: String },
    /// Not reached because a previous event failed.
    NotReached,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Similar to [`EventStatus`], but for [`SubEvent`] (which can oly succeed or fail).
pub enum SubEventStatus {
    /// The event succeeded.
    /// State changes are committed.
    Ok(SubEvent),
    /// The event failed.
    /// State changes are reverted.
    Failed(SubEvent),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandleEventStatus {
    /// The event succeeded.
    /// State changes are committed.
    Ok(Event),
    /// The event failed.
    /// State changes are reverted.
    Failed { event: Event, error: String },
    /// The event failed but the error was handled.
    /// State changes are reverted but the tx continues.
    FailedButHandled { event: Event, error: String },
}

impl HandleEventStatus {
    pub fn failed<E>(event: Event, error: E) -> Self
    where
        E: ToString,
    {
        Self::Failed {
            event,
            error: error.to_string(),
        }
    }

    pub fn failed_but_handled<E>(event: Event, error: E) -> Self
    where
        E: ToString,
    {
        Self::FailedButHandled {
            event,
            error: error.to_string(),
        }
    }
}
