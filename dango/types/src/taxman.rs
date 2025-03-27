use {
    grug::{Addr, Coins, Denom, NumberConst, Timestamp, Udec128, Uint128},
    std::collections::BTreeMap,
};

#[grug::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
}

#[grug::derive(Serde, Borsh)]
/// A fee payment including the coins paid and the USD value of the coins.
pub struct FeePayments {
    /// The coins paid.
    pub coins: Coins,
    /// The USD value of the coins.
    pub usd_value: Uint128,
}

impl Default for FeePayments {
    fn default() -> Self {
        Self {
            coins: Coins::new(),
            usd_value: Uint128::ZERO,
        }
    }
}

#[grug::derive(Serde, Borsh)]
#[derive(Copy, PartialOrd, Ord)]
pub enum FeeType {
    /// Gas Fee.
    Gas,
    /// Protocol fee for maker trades in Dango DEX.
    Maker,
    /// Protocol fee for taker trades in Dango DEX.
    Taker,
    /// Fee for bridging assets out of Dango chain.
    Withdraw,
}

impl FeeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeeType::Gas => "gas",
            FeeType::Maker => "maker",
            FeeType::Taker => "taker",
            FeeType::Withdraw => "withdraw",
        }
    }
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the fee configurations.
    /// Can only be called by the chain's owner.
    Configure { new_cfg: Config },
    /// Forward protocol fee to the taxman.
    Pay {
        payments: BTreeMap<Addr, (FeeType, Coins)>,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},

    /// Returns the total amount of fees collected from a user since the
    /// specified timestamp.
    #[returns(FeePayments)]
    FeesForUser {
        /// The user to query fees for.
        user: Addr,
        /// The type of fee to query. If not provided, the total amount of fees
        /// collected for all fee types will be returned.
        fee_type: Option<FeeType>,
        /// The start timestamp to query fees for. If not provided, the total
        /// amount of fees collected will be returned.
        since: Option<Timestamp>,
    },
}

#[grug::derive(Serde)]
#[grug::event("receive_fee")]
pub struct ReceiveFee {
    /// The Dango smart contract that handled this fee.
    pub handler: Addr,
    pub user: Addr,
    #[serde(rename = "type")]
    pub ty: FeeType,
    pub amount: Coins,
}
