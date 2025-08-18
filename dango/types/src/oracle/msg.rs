use {
    crate::oracle::{PrecisionedPrice, PriceSource},
    grug::Denom,
    pyth_types::{GuardianSet, GuardianSetIndex, PriceUpdate},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_sets: BTreeMap<GuardianSetIndex, GuardianSet>,
    pub price_sources: BTreeMap<Denom, PriceSource>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set the price sources for the given denoms.
    RegisterPriceSources(BTreeMap<Denom, PriceSource>),
    /// Submit price data from Pyth Network.
    FeedPrices(PriceUpdate),
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
    GuardianSet { index: GuardianSetIndex },
    /// Enumerate the guardian set of all known indexed.
    #[returns(BTreeMap<GuardianSetIndex, GuardianSet>)]
    GuardianSets {
        start_after: Option<GuardianSetIndex>,
        limit: Option<u32>,
    },
}
