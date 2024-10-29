use {
    super::{GuardianSetInfo, PythId, PythVaa},
    pyth_sdk::PriceFeed,
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub guardian_set: BTreeMap<u32, GuardianSetInfo>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    UpdatePriceFeeds { data: Vec<PythVaa> },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(PriceFeed)]
    PriceFeed { id: PythId },
}
