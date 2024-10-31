use {
    super::{GuardianSetInfo, PrecisionedPrice, PriceSourceCollector, PythVaa},
    grug::Denom,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_set: BTreeMap<u32, GuardianSetInfo>,
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
}
