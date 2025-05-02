use {
    grug::{Addr, Coins, Denom, Json, Message, MsgExecute, Part, btree_map},
    std::{collections::BTreeMap, sync::LazyLock},
};

pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("all"));

#[grug::derive(Serde)]
pub enum Action {
    /// Execute a contract with the output tokens as funds.
    Execute { contract: Addr, msg: Json },
    /// Send the output tokens to a recipient.
    Transfer(Addr),
}

impl Action {
    pub fn into_message(self, funds: Coins) -> Message {
        match self {
            Action::Execute { contract, msg } => Message::Execute(MsgExecute {
                contract,
                msg,
                funds,
            }),
            Action::Transfer(recipient) => Message::Transfer(btree_map! { recipient => funds }),
        }
    }
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    // underlying_denom => alloyed subdenom
    //
    // e.g. `hyp/eth/usdc` => `usdc` means `hyp/eth/usdc` is to be alloyed as
    // part of `all/usdc`.
    pub mapping: BTreeMap<Denom, Part>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set a new underlying denom to alloyed subdenom mapping.
    ///
    /// Can only be called by the chain owner.
    ///
    /// Not that this is append-only, meaning you can change or remove an
    /// existing mapping.
    SetMapping(BTreeMap<Denom, Part>),
    /// Convert underlying tokens to alloyed tokens.
    Alloy {
        /// An action to be execute after alloying.
        /// If unspecified, the alloyed tokens are sent back to the caller.
        and_then: Option<Action>,
    },
    /// Convert alloyed tokens to underlying tokens.
    Dealloy {
        /// An action to be execute after dealloying.
        /// If unspecified, the underlying tokens are sent back to the caller.
        and_then: Option<Action>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Convert a single underlying denom to alloyed denom.
    #[returns(Option<Denom>)]
    Alloy { underlying_denom: Denom },
    /// Enumerate all underlying denoms to alloyed denoms conversions.
    #[returns(BTreeMap<Denom, Denom>)]
    Alloys {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Convert a single alloyed denom to underlying denom.
    #[returns(Option<Denom>)]
    Dealloy { alloyed_denom: Denom },
    /// Enumerate all alloyed denoms to underlying denoms conversions.
    #[returns(BTreeMap<Denom, Denom>)]
    Dealloys {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
