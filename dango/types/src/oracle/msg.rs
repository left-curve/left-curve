use {
    super::{GuardianSetInfo, PrecisionedPrice, PriceSource, PythVaa},
    grug::Denom,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_sets: BTreeMap<u32, GuardianSetInfo>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Set the price sources for the given denoms.
    RegisterPriceSources(BTreeMap<Denom, PriceSource>),
    /// Submit price data from Pyth Network.
    FeedPrices(Vec<PythVaa>),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(PrecisionedPrice)]
    QueryPrice { denom: Denom },

    #[returns(BTreeMap<Denom, PriceSource>)]
    QueryPriceSources {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
