use grug::{Addr, Denom, Udec128};

#[grug::derive(Serde, Borsh)]
pub struct Config {
    pub fee_recipient: Addr,
    pub fee_denom: Denom,
    pub fee_rate: Udec128,
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the fee configurations.
    /// Can only be called by the chain's owner.
    UpdateConfig { new_cfg: Config },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},
}
