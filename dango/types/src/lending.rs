use {
    grug::{Addr, Bound, Bounded, Bounds, Coins, Denom, NumberConst, Part, Udec128},
    std::{collections::BTreeMap, sync::LazyLock},
};

/// The namespace that tokens associated with lending will be minted under.
/// The lending contract must be granted admin power over this namespace.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("lending"));

/// Sub-namespace that liquidity share tokens will be minted under.
pub static SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("pool"));

// -------------------------------- Bounds -------------------------------------

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

/// Configurations and state of a market.
#[grug::derive(Serde, Borsh)]
pub struct Market {
    // TODO
}

/// A set of updates to be applied to a market.
#[grug::derive(Serde)]
pub struct MarketUpdates {
    // TODO
}

// -------------------------------- Messages -----------------------------------
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub markets: BTreeMap<Denom, MarketUpdates>,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Apply updates to markets.
    UpdateMarkets(BTreeMap<Denom, MarketUpdates>),
    /// Deposit tokens into the lending pool.
    /// Sender must attach one or more supported tokens and nothing else.
    Deposit {},
    /// Withdraw tokens from the lending pool by redeeming LP tokens.
    /// Sender must attach one or more LP tokens and nothing else.
    Withdraw {},
    /// Borrow coins from the lending pool.
    /// Sender must be a margin account.
    Borrow(Coins),
    /// Repay debt.
    /// Sender must be a margin account.
    Repay {},
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
}
