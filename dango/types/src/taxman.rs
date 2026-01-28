use {
    crate::account_factory::UserIndex,
    core::str,
    grug::{
        Addr, Bounded, Coins, Denom, Timestamp, Udec128, ZeroInclusiveOneExclusive,
        ZeroInclusiveOneInclusive,
    },
    std::collections::BTreeMap,
};

pub type ShareRatio = Bounded<Udec128, ZeroInclusiveOneInclusive>;
pub type CommissionRebund = Bounded<Udec128, ZeroInclusiveOneExclusive>;

pub type Referrer = UserIndex;
pub type Referee = UserIndex;

#[grug::derive(Serde, Borsh)]
#[derive(Default)]
pub struct UserReferralData {
    /// Total trading volume made by the user (USD).
    pub volume: Udec128,
    /// Total commission rebounded to the user (USD).
    pub commission_rebounded: Udec128,
    /// Total number of referees referred by the user.
    pub referee_count: u32,
    /// Total trading volume made by the user's direct referees (USD).
    pub referee_volume: Udec128,
    /// Total commission rebounded to the user's direct referees (USD).
    pub referees_commission_rebounded: Udec128,
}

#[grug::derive(Serde, Borsh)]
pub struct RefereeData {
    /// Timestamp when the referee registered with the referral.
    pub registered_at: Timestamp,
    /// Total trading volume made by the referee (USD).
    pub volume: Udec128,
    /// Total commission rebounded to the referrer (USD).
    pub commission_rebounded: Udec128,
}

pub struct ReferrerInfo {
    /// User index of the referrer.
    pub user: UserIndex,
    /// The commission rebund ratio is how much commission the referrer will receive from the fees.
    /// This depends on the direct referees volume in the last 30 days and also on the commission rebund
    /// of the previous referrers in the chain.
    pub commission_rebund: CommissionRebund,
    /// The share ratio is how much of the commission rebounded to the referrer
    /// want to split with the referee.
    pub share_ratio: ShareRatio,
}

#[grug::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
}

#[grug::derive(Serde)]
#[derive(Copy)]
pub enum FeeType {
    /// Gas Fee.
    Gas,
    /// Protocol fee for trading in Dango DEX.
    ///
    /// Not to be confused with liquidity fee, which is paid to liquidity
    /// providers when using Dango DEX's instant swap feature.
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
        #[serde(rename = "type")]
        ty: FeeType,
        payments: BTreeMap<Addr, Coins>,
    },
    /// Callable by:
    /// 1. the account factory, when a user registers with a referral code;
    /// 2. a user, if he didn't provide a referral code when registering.
    ///    However, if he did provide one when registering, it can't be changed.
    SetReferral {
        referrer: Referrer,
        referee: Referee,
    },
    /// Callable by referrers.
    /// NOTE: Can only increase, not decrease. Prevent referrers from rugging referees.
    SetFeeShareRatio(ShareRatio),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the fee configurations.
    #[returns(Config)]
    Config {},
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

#[grug::derive(Serde)]
#[grug::event("referral")]
pub struct Referral {
    pub referrer: Referrer,
    pub referee: Referee,
}
