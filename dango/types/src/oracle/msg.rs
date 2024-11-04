use {
    super::{GuardianSetInfo, PrecisionedPrice, PriceSourceCollector, PythVaa},
    grug::Denom,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_sets: BTreeMap<u32, GuardianSetInfo>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    UpdatePriceFeeds {
        data: Vec<PythVaa>,
    },
    RegisterDenom {
        denom: Denom,
        price_source: PriceSourceCollector,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(PrecisionedPrice)]
    QueryPrice { denom: Denom },

    #[returns(BTreeMap<Denom, PriceSourceCollector>)]
    QueryPriceSources {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}
