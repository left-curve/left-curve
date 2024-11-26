use {
    crate::account::margin::CollateralPower,
    grug::{Addr, Bounded, Denom, Udec128, ZeroExclusiveOneExclusive},
    std::{collections::BTreeMap, sync::LazyLock},
};

/// Denomination of the Dango token.
pub static DANGO_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::new_unchecked(["udng"]));

/// Application-specific configurations of the Dango chain.
#[grug::derive(Serde)]
pub struct AppConfig {
    /// Addresses of relevant Dango contracts.
    pub addresses: AppAddresses,
    /// The powers of all collateral tokens. This is the adjustment factor for
    /// the collateral value of a given collateral token. Meaning, if the
    /// collateral token has a power of 0.9, then the value of the collateral
    /// token is 90% of its actual value.
    pub collateral_powers: BTreeMap<Denom, CollateralPower>,

    /// The margin account utilization rate down to which an account can be liquidated.
    /// E.g. if this is set to 0.9, then as soon as the account's utilization rate reaches 1.0
    /// and becomes liquidatable, liquidators can pay off the accounts debts (in return for some of
    /// its collateral) until the account's utilization rate is at this value.
    pub target_utilization_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub ibc_transfer: Addr,
    pub lending: Addr,
    pub oracle: Addr,
}
