use {
    grug::{Addr, Coins, Denom},
    std::collections::BTreeMap,
};

/// The namespace that lending pool uses.
pub const NAMESPACE: &str = "lending";

/// Configurations and state of a market.
#[grug::derive(Serde, Borsh)]
pub struct Market {
    // TODO
}

/// A set of updates to be applied to a market.
#[grug::derive(Serde)]
pub struct MarketUpdates {
    // TODO
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub markets: BTreeMap<Denom, MarketUpdates>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Apply updates to markets.
    UpdateMarkets(BTreeMap<Denom, MarketUpdates>),

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
    /// Query the lending market of a single token.
    #[returns(Market)]
    Market { denom: Denom },
    /// Enumerate all lending markets.
    #[returns(BTreeMap<Denom, Market>)]
    Markets {
        start_after: Option<Denom>,
        limit: Option<u32>,
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
