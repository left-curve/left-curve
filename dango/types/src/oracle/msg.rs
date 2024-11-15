use {
    super::{GuardianSet, PrecisionedPrice, PriceSource},
    grug::{Binary, Denom, NonEmpty},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_sets: BTreeMap<u32, GuardianSet>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set the price sources for the given denoms.
    RegisterPriceSources(BTreeMap<Denom, PriceSource>),
    /// Submit price data from Pyth Network.
    FeedPrices(NonEmpty<Vec<Binary>>),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the price of the given denom.
    #[returns(PrecisionedPrice)]
    Price { denom: Denom },
    /// Enumerate the prices of all supported denoms.
    #[returns(BTreeMap<Denom, PrecisionedPrice>)]
    Prices {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the price source of the given denom.
    #[returns(PriceSource)]
    PriceSource { denom: Denom },
    /// Enumerate the price sources of all supported denoms.
    #[returns(BTreeMap<Denom, PriceSource>)]
    PriceSources {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the guardian set of the given index.
    #[returns(GuardianSet)]
    GuardianSet { index: u32 },
    /// Enumerate the guardian set of all known indexed.
    #[returns(BTreeMap<u32, GuardianSet>)]
    GuardianSets {
        start_after: Option<u32>,
        limit: Option<u32>,
    },
}
