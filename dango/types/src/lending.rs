use {
    grug::{Addr, Coins, Denom},
    std::collections::BTreeSet,
};

/// The namespace that lending pool uses.
pub const NAMESPACE: &str = "lending";

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub whitelisted_denoms: BTreeSet<Denom>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Whitelist a denom. Can only be invoked by the owner.
    WhitelistDenom(Denom),

    /// Delist a denom. Can only be invoked by the owner.
    DelistDenom(Denom),

    /// Deposit tokens into the lending pool.
    Deposit {},

    /// Withdraw tokens from the lending pool by redeeming LP tokens. LP tokens
    /// should be sent to the contract together with this message.
    Withdraw {},

    /// Borrow coins from the lending pool. Can only be invoked by margin
    /// accounts.
    Borrow {
        /// The coins to borrow.
        coins: Coins,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Get the list of whitelisted denoms.
    #[returns(Vec<Denom>)]
    WhitelistedDenoms {
        /// The maximum number of denoms to return. If not set, will attempt to
        /// return all denoms.
        limit: Option<u32>,

        /// The denom to start paginating after. If not set, will start from the
        /// first denom.
        start_after: Option<Denom>,
    },

    /// Get the debts of a margin account.
    #[returns(Coins)]
    DebtsOfAccount(Addr),

    /// Paginate over all the lending pool's liabilities. Returns a Vec with
    /// tuples of (Addr, Coins), where the Addr is the address of the account
    /// that owes the debt and the Coins are the coins owed.
    #[returns(Vec<(Addr, Coins)>)]
    Liabilities {
        /// The maximum number of entries to return. If not set, will attempt to
        /// return all entries.
        limit: Option<u32>,

        /// The address to start paginating after. If not set, will start from
        /// the first address.
        start_after: Option<Addr>,
    },
}
