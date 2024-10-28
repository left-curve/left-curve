#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {}
