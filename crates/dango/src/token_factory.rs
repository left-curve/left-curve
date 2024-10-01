use {
    crate::account_factory::Username,
    grug::{Addr, Coin, Denom, Uint256},
    std::collections::BTreeMap,
};

/// The namespace that token factory uses.
pub const NAMESPACE: &str = "factory";

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub denom_creation_fee: Coin,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
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
        amount: Uint256,
    },
    /// Burn the token of the specified subdenom and amount from a source.
    Burn {
        denom: Denom,
        from: Addr,
        amount: Uint256,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the denom creation fee.
    #[returns(Coin)]
    DenomCreationFee {},
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
