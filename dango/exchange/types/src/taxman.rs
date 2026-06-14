use {dango_math::Udec128, dango_primitives::Denom};

#[dango_primitives::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
}

#[dango_primitives::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[dango_primitives::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the fee configurations.
    /// Can only be called by the chain's owner.
    Configure { new_cfg: Config },
}

#[dango_primitives::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},
}
