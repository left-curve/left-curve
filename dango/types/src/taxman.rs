use {
    grug_math::Udec128,
    grug_types::{Addr, Coins, Denom},
    std::collections::BTreeMap,
};

#[grug_types::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
}

#[grug_types::derive(Serde)]
#[derive(Copy)]
pub enum FeeType {
    /// Gas Fee.
    Gas,
    /// Protocol fee for trading.
    Trade,
    /// Fee for bridging assets out of Dango chain.
    Withdraw,
}

impl FeeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeeType::Gas => "gas",
            FeeType::Trade => "trade",
            FeeType::Withdraw => "withdraw",
        }
    }
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
    /// Forward protocol fee to the taxman.
    Pay {
        #[serde(rename = "type")]
        ty: FeeType,
        payments: BTreeMap<Addr, Coins>,
    },
}

#[grug_types::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},
}

#[grug_types::derive(Serde)]
#[grug_types::event("receive_fee")]
pub struct ReceiveFee {
    /// The Dango smart contract that handled this fee.
    pub handler: Addr,
    pub user: Addr,
    #[serde(rename = "type")]
    pub ty: FeeType,
    pub amount: Coins,
}
