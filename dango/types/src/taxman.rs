use {grug_math::Udec128, grug_types::Denom};

#[grug_types::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
}

#[grug_types::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the fee configurations.
    /// Can only be called by the chain's owner.
    Configure { new_cfg: Config },
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},
}
