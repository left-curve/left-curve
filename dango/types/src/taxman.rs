use {
    crate::account_factory::UserIndex,
    core::str,
    grug::{
        Addr, Bounded, Coins, Denom, NumberConst, Timestamp, Udec128, Udec128_6, Uint128,
        ZeroInclusiveOneExclusive, ZeroInclusiveOneInclusive,
    },
    std::collections::BTreeMap,
};

pub type ShareRatio = Bounded<Udec128, ZeroInclusiveOneInclusive>;
pub type CommissionRebund = Bounded<Udec128, ZeroInclusiveOneExclusive>;

pub type Referrer = UserIndex;
pub type Referee = UserIndex;

#[grug::derive(Serde, Borsh)]
#[derive(Default)]
/// Store all the cumulative data for an user related to the referral program.
pub struct UserReferralData {
    /// Total trading volume made by the user (USD).
    pub volume: Udec128,
    /// Total commission rebounded to the user (USD).
    pub commission_rebounded: Udec128,
    /// Total number of referees referred by the user.
    pub referee_count: u32,
    /// Total trading volume made by the user's direct referees (USD).
    pub referees_volume: Udec128,
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

pub struct ReferralConfig {
    /// Minimum volume required for a user to become a referrer.
    pub volume_to_be_referrer: Uint128,
    /// Default commission rebund ratio, applied when no volume thresholds are met.
    pub commission_rebound_default: CommissionRebund,
    /// Mapping from volume thresholds to commission rebund ratios.
    pub commission_rebound_by_volume: BTreeMap<Uint128, CommissionRebund>,
}

impl Default for ReferralConfig {
    fn default() -> Self {
        Self {
            volume_to_be_referrer: Default::default(),
            commission_rebound_default: CommissionRebund::new_unchecked(Udec128::ZERO),
            commission_rebound_by_volume: Default::default(),
        }
    }
}

#[grug::derive(Serde, Borsh)]
pub struct Config {
    pub fee_denom: Denom,
    /// Units of the fee token for each unit of gas consumed.
    pub fee_rate: Udec128,
    // Config for the referral program.
    pub referral: ReferralConfig,
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
    /// Report trading volumes of users.
    /// Can only be called by the spot and perp DEX contracts.
    ReportVolumes(BTreeMap<Addr, Udec128_6>),
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
    /// Returns the trading volume of a user since the specified timestamp.
    #[returns(Udec128)]
    VolumeByUser {
        /// The user to query trading volume for.
        user: UserIndex,
        /// The start timestamp to query trading volume for. If not provided,
        /// user's total trading volume since genesis will be returned.
        since: Option<Timestamp>,
    },
    /// Query the referref of the user.
    #[returns(Option<Referrer>)]
    Referrer { user: Referee },
    /// Query the stats of an user for the referral program.
    #[returns(UserReferralData)]
    ReferralStats { user: UserIndex },
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
