use {
    crate::account::margin::CollateralPower,
    grug::{Addr, Denom},
    std::collections::BTreeMap,
};

/// Application-specific configurations of the Dango chain.
#[grug::derive(Serde)]
pub struct AppConfig {
    pub addresses: AppAddresses,
    /// The powers of all collateral tokens. This is the adjustment factor for
    /// the collateral value of a given collateral token. Meaning, if the
    /// collateral token has a power of 0.9, then the value of the collateral
    /// token is 90% of its actual value.
    pub collateral_powers: BTreeMap<Denom, CollateralPower>,
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub ibc_transfer: Addr,
    pub lending: Addr,
    pub oracle: Addr,
}
