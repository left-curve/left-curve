use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug::{Addr, Bound, Bounded, Bounds, Coins, Denom, NumberConst, Part, Udec128},
    optional_struct::optional_struct,
    serde::{Deserialize, Serialize},
    std::{collections::BTreeMap, sync::LazyLock},
};

/// The namespace that tokens associated with lending will be minted under.
/// The lending contract must be granted admin power over this namespace.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("lending"));

/// Sub-namespace that liquidity share tokens will be minted under.
pub static SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("pool"));

// -------------------------------- Bounds -------------------------------------

/// Defines the bounds for a loan-to-value ratio: 0 < LoanToValue < 1.
#[grug::derive(Serde)]
pub struct LoanToValueBounds;
impl Bounds<Udec128> for LoanToValueBounds {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}
/// A decimal bounded by the loan-to-value bounds.
pub type LoanToValue = Bounded<Udec128, LoanToValueBounds>;

/// Defines the bounds for a collateral power: 0 < CollateralPower <= 1.
#[grug::derive(Serde)]
pub struct CollateralPowerBounds;
impl Bounds<Udec128> for CollateralPowerBounds {
    const MAX: Option<Bound<Udec128>> = Some(Bound::Inclusive(Udec128::ONE));
    const MIN: Option<Bound<Udec128>> = Some(Bound::Exclusive(Udec128::ZERO));
}
/// A decimal bounded by the collateral power bounds.
pub type CollateralPower = Bounded<Udec128, CollateralPowerBounds>;

// -------------------------------- Market -------------------------------------

/// Configurations and state of a market (borrowable assets).
#[optional_struct(MarketUpdates)]
#[derive(
    Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct Market {
    // TODO
}

// -------------------------------- Messages -----------------------------------
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub markets: BTreeMap<Denom, Market>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Apply updates to markets.
    UpdateMarkets(BTreeMap<Denom, MarketUpdates>),
    /// Set the collateral power for a denom.
    SetCollateralPower {
        denom: Denom,
        power: CollateralPower,
    },
    /// Delist a collateral token. Removes it from the collateral power map.
    DelistCollateral { denom: Denom },
    /// Deposit tokens into the lending pool.
    /// Sender must attach one or more supported tokens and nothing else.
    Deposit {},
    /// Withdraw tokens from the lending pool by redeeming LP tokens.
    /// Sender must attach one or more LP tokens and nothing else.
    Withdraw {},
    /// Borrow coins from the lending pool.
    /// Sender must be a margin account.
    Borrow(Coins),
}

#[grug::derive(Serde)]
pub struct HealthResponse {
    /// The margin account's utilization rate.
    pub utilization_rate: Udec128,
    /// The total value of the margin account's debt.
    pub total_debt_value: Udec128,
    /// The total value of the margin account's collateral, adjusted for
    /// the collateral power of each denom.
    pub total_adjusted_collateral_value: Udec128,
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the lending market of a single token.
    #[returns(Market)]
    Market { denom: Denom },
    /// Enumerate all lending markets.
    #[returns(BTreeMap<Denom, Market>)]
    Markets {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// Query the debt of a single margin account.
    #[returns(Coins)]
    Debt { account: Addr },
    /// Enumerate debts of all margin accounts.
    #[returns(BTreeMap<Addr, Coins>)]
    Debts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// Returns all collateral powers.
    #[returns(BTreeMap<Denom, CollateralPower>)]
    CollateralPowers {},
    /// Queries the health of a margin account.
    #[returns(HealthResponse)]
    Health { account: Addr },
}
