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
    /// A contract emitted a custom event.
    Guest(EvtGuest),
    // TODO: IBC events
}

impl Event {
    pub fn configure(sender: Addr) -> Self {
        Self::Configure(EvtConfigure { sender })
    }

    // pub fn transfer(sender: Addr, recipient: Addr, coins: Coins) -> Self {
    //     Self::Transfer(EvtTransfer {
    //         sender,
    //         recipient,
    //         coins,
    //     })
    // }

    pub fn upload(sender: Addr, code_hash: Hash256) -> Self {
        Self::Upload(EvtUpload { sender, code_hash })
    }

    pub fn instantiate(
        sender: Addr,
        contract: Addr,
        code_hash: Hash256,
        label: Option<Label>,
        admin: Option<Addr>,
        transfer_event: Option<EvtTransfer>,
        guest_event: EvtGuest,
    ) -> Self {
        Self::Instantiate(EvtInstantiate {
            sender,
            contract,
            code_hash,
            label,
            admin,
            transfer_event,
            guest_event,
        })
    }

    pub fn execute(
        sender: Addr,
        contract: Addr,
        funds: Coins,
        transfer_event: Option<EvtTransfer>,
        guest_event: EvtGuest,
    ) -> Self {
        Self::Execute(EvtExecute {
            sender,
            contract,
            transfer_event,
            guest_event,
            funds,
        })
    }

    pub fn migrate(
        sender: Addr,
        contract: Addr,
        old_code_hash: Hash256,
        new_code_hash: Hash256,
        guest_event: EvtGuest,
    ) -> Self {
        Self::Migrate(EvtMigrate {
            sender,
            contract,
            old_code_hash,
            new_code_hash,
            guest_event,
        })
    }

    pub fn reply(contract: Addr, reply_on: ReplyOn, guest_event: EvtGuest) -> Self {
        Self::Reply(EvtReply {
            contract,
            reply_on: ReplyOnDiscriminants::from(reply_on),
            guest_event,
        })
    }

    pub fn authenticate(sender: Addr, backrun_requested: bool, guest_event: EvtGuest) -> Self {
        Self::Authenticate(EvtAuthenticate {
            sender,
            backrun: backrun_requested,
            guest_event,
        })
    }

    pub fn backrun(sender: Addr, guest_event: EvtGuest) -> Self {
        Self::Backrun(EvtBackrun {
            sender,
            guest_event,
        })
    }

    pub fn withhold(sender: Addr, taxman: Addr, guest_event: EvtGuest) -> Self {
        Self::Withhold(EvtWithhold {
            sender,
            taxman,
            guest_event,
        })
    }

    pub fn finalize(sender: Addr, taxman: Addr, guest_event: EvtGuest) -> Self {
        Self::Finalize(EvtFinalize {
            sender,
            taxman,
            guest_event,
        })
    }

    pub fn cron(contract: Addr, time: Timestamp, next: Timestamp, guest_event: EvtGuest) -> Self {
        Self::Cron(EvtCron {
            contract,
            time,
            next,
            guest_event,
        })
    }

    // pub fn guest(contract: Addr, name: String, sub_events: Vec<ContractEvent>) -> Self {
    //     Self::Guest(EvtGuest {
    //         contract,
    //         method: name,
    //         contract_events: sub_events,
    //     })
    // }

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
    pub bank_guest: EvtGuest,
    pub receive_guest: Option<EvtGuest>,
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
    pub transfer_event: Option<EvtTransfer>,
    pub guest_event: EvtGuest,
    // TODO: is it necessary to include the InstantiateMsg?
}

/// An event indicating that a contract was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtExecute {
    pub sender: Addr,
    pub contract: Addr,
    pub funds: Coins,
    pub transfer_event: Option<EvtTransfer>,
    pub guest_event: EvtGuest,
    // TODO: is it necessary to include the ExecuteMsg?
}

/// An event indicating that a contract was migrated to a new code hash.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtMigrate {
    pub sender: Addr,
    pub contract: Addr,
    pub old_code_hash: Hash256,
    pub new_code_hash: Hash256,
    pub guest_event: EvtGuest,
    // TODO: is it necessary to include the MigrateMsg?
}

/// An event indicating that a contract was replied the outcome of its submessage.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtReply {
    pub contract: Addr,
    pub reply_on: ReplyOnDiscriminants,
    pub guest_event: EvtGuest,
}

/// An event indicating that a contract authenticated a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtAuthenticate {
    pub sender: Addr,
    pub backrun: bool,
    pub guest_event: EvtGuest,
}

/// An event indicating that a contract backran a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtBackrun {
    pub sender: Addr,
    pub guest_event: EvtGuest,
}

/// An event indicating that The taxman withheld the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtWithhold {
    pub sender: Addr,
    pub taxman: Addr,
    pub guest_event: EvtGuest,
}

/// An event indicating that the taxman finalized the fee for a transaction.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtFinalize {
    pub sender: Addr,
    pub taxman: Addr,
    pub guest_event: EvtGuest,
}

/// An event indicating that a cronjob was executed.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtCron {
    pub contract: Addr,
    /// The timestamp of this cronjob execution.
    pub time: Timestamp,
    /// The timestamp of the next cronjob execution is scheduled.
    pub next: Timestamp,
    pub guest_event: EvtGuest,
}

/// An event indicating that a contract emitted a custom event.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EvtGuest {
    pub contract: Addr,
    /// The wasm export function that was being called when the event was emitted.
    pub method: String,
    /// Sub events emitted by the contract.
    pub contract_events: Vec<ContractEvent>,
    pub sub_events: Vec<Event>,
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     const HASH: Hash256 = Hash256::from_inner([0; 32]);
//     const ADDR: Addr = Addr::mock(1);

//     #[test]
//     fn variant_name() {
//         for (event, name) in [
//             (Event::configure(ADDR), "configure"),
//             (Event::transfer(ADDR, ADDR, Coins::default()), "transfer"),
//             (Event::upload(ADDR, HASH), "upload"),
//             (
//                 Event::instantiate(ADDR, Addr::mock(1), HASH, None, None),
//                 "instantiate",
//             ),
//             (Event::execute(ADDR, Addr::mock(1)), "execute"),
//             (Event::migrate(ADDR, Addr::mock(1), HASH, HASH), "migrate"),
//             (Event::reply(ADDR, ReplyOn::Never), "reply"),
//             (Event::authenticate(ADDR, false), "authenticate"),
//             (Event::backrun(ADDR), "backrun"),
//             (Event::withhold(ADDR, ADDR), "withhold"),
//             (Event::finalize(ADDR, ADDR), "finalize"),
//             (
//                 Event::cron(ADDR, Timestamp::default(), Timestamp::default()),
//                 "cron",
//             ),
//             (Event::guest(ADDR, "method".to_string(), vec![]), "guest"),
//         ] {
//             assert_eq!(event.variant_name(), name);
//         }
//     }
// }
