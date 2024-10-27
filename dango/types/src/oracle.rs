use {
    grug::{Addr, Denom, Timestamp, Udec128},
    std::collections::{BTreeMap, BTreeSet},
};

#[grug::derive(Serde, Borsh)]
pub struct Config {
    /// The minimum number of guardians that must have voted on the price in
    /// order for it to be considered valid and votes tallied.
    pub quorum: u32,
}

#[grug::derive(Serde, Borsh)]
pub struct Price {
    pub price: Udec128,
    /// The time this price was computed.
    ///
    /// Contracts that query the price should check this timestamp to make sure
    /// it's not too old before using it in business logic.
    pub timestamp: Timestamp,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
    pub guardians: BTreeSet<Addr>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Provides price feed for a list of denoms. Called by guardians.
    FeedPrices(BTreeMap<Denom, Udec128>),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the oracle configurations.
    #[returns(Config)]
    Config {},
    /// Query the guardian set.
    #[returns(BTreeSet<Addr>)]
    Guardians {},
    /// Query the price of a specific denom.
    #[returns(Price)]
    Price { denom: Denom },
    /// Enumerate the prices of all denoms.
    #[returns(BTreeMap<Denom, Price>)]
    Prices {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the price feed of a given denom from a specific guardian.
    #[returns(Udec128)]
    PriceFeed { denom: Denom, guardian: Addr },
    /// Enumerate the price feeds of a denom from all guardians.
    #[returns(BTreeMap<Addr, Udec128>)]
    PriceFeeds {
        denom: Denom,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}
