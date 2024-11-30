use {
    crate::{Addr, Coins, Hash256, Json, Label, ReplyOn, Timestamp},
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
    /// A contract emitted a custom event.
    Guest(EvtGuest),
    // TODO: IBC events
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
    // TODO: is it necessary to include the InstantiateMsg?
}

/// An event indicating that a contract was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtExecute {
    pub sender: Addr,
    pub contract: Addr,
    // TODO: is it necessary to include the ExecuteMsg?
}

/// An event indicating that a contract was migrated to a new code hash.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtMigrate {
    pub sender: Addr,
    pub contract: Addr,
    pub old_code_hash: Hash256,
    pub new_code_hash: Hash256,
    // TODO: is it necessary to include the MigrateMsg?
}

/// An event indicating that a contract was replied the outcome of its submessage.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtReply {
    pub contract: Addr,
    pub reply_on: ReplyOn,
}

/// An event indicating that a contract authenticated a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtAuthenticate {
    pub sender: Addr,
    pub backrun_requested: bool,
}

/// An event indicating that a contract backran a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtBackrun {
    pub sender: Addr,
}

/// An event indicating that The taxman withheld the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtWithhold {
    pub sender: Addr,
    pub taxman: Addr,
}

/// An event indicating that the taxman finalized the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtFinalize {
    pub sender: Addr,
    pub taxman: Addr,
}

/// An event indicating that a cronjob was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtCron {
    pub contract: Addr,
    /// The timestamp of this cronjob execution.
    pub time: Timestamp,
    /// The timestamp of the next cronjob execution is scheduled.
    pub next: Timestamp,
}

/// An event indicating that a contract emitted a custom event.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub name: String,
    /// A string chosen by the contract to identify the event's type.
    #[serde(rename = "type")]
    pub ty: String,
    /// Arbitrary data emitted by the contract.
    pub data: Json,
}
