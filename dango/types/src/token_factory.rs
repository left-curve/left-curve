use {
    crate::{account_factory::Username, bank::Metadata},
    grug::{Addr, Coin, Denom, NonZero, Part, Uint128},
    std::{collections::BTreeMap, sync::LazyLock},
};

/// The namespace that token factory uses.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("factory"));

#[grug::derive(Serde, Borsh)]
pub struct Config {
    /// A one time, flat fee for creating a denom.
    ///
    /// It's optional, but if provided, must be non-zero.
    pub token_creation_fee: Option<NonZero<Coin>>,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the configurations.
    /// Can only be called by the chain owner.
    Configure { new_cfg: Config },
    /// Create a new token with the given sub-denomination, and appoint an admin
    /// who can mint or burn this token.
    ///
    /// The creator must attach exactly the amount of denom creation fee along
    /// with the call.
    Create {
        subdenom: Denom,
        // If provided, the denom will be formatted as:
        // > factory/{username}/{subdenom}
        // Otherwise, it will be formatted as:
        // > factory/{sender_address}/{subdenom}
        username: Option<Username>,
        // If not provided, use the message sender's address.
        admin: Option<Addr>,
    },
    /// Mint the token of the specified subdenom and amount to a recipient.
    Mint {
        denom: Denom,
        to: Addr,
        amount: Uint128,
    },
    /// Burn the token of the specified subdenom and amount from a source.
    Burn {
        denom: Denom,
        from: Addr,
        amount: Uint128,
    },
    /// Set [`Metadata`] for a denom in `bank` contract.
    SetMetadata { denom: Denom, metadata: Metadata },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the token factory's configurations.
    #[returns(Config)]
    Config {},
    /// Query a denom's admin address.
    #[returns(Addr)]
    Admin { denom: Denom },
    /// Enumerate all denoms and their admin addresses.
    #[returns(BTreeMap<Denom, Addr>)]
    Admins {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
