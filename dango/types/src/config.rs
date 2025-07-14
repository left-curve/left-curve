use {
    crate::account::margin::CollateralPower,
    grug::{Addr, Bounded, Denom, Udec128, ZeroExclusiveOneExclusive, ZeroInclusiveOneExclusive},
    std::collections::BTreeMap,
};

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
    /// The minimum liquidation bonus that liquidators receive when liquidating an
    /// undercollateralized margin account.
    /// The liquidation bonus is defined as a percentage of the repaid debt value.
    pub min_liquidation_bonus: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The maximum liquidation bonus that liquidators receive when liquidating an
    /// undercollateralized margin account.
    pub max_liquidation_bonus: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// The margin account utilization rate down to which an account can be liquidated.
    /// E.g. if this is set to 0.9, then as soon as the account's utilization rate reaches 1.0
    /// and becomes liquidatable, liquidators can pay off the accounts debts (in return for some of
    /// its collateral) until the account's utilization rate is at this value.
    pub target_utilization_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
    /// The maker fee for the DEX.
    pub maker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The taker fee for the DEX.
    pub taker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            addresses: Default::default(),
            collateral_powers: Default::default(),
            target_utilization_rate: Bounded::new(Udec128::new_percent(90)).unwrap(),
            min_liquidation_bonus: Bounded::new(Udec128::new_percent(2)).unwrap(),
            max_liquidation_bonus: Bounded::new(Udec128::new_percent(20)).unwrap(),
            maker_fee_rate: Bounded::new(Udec128::new_bps(25)).unwrap(),
            taker_fee_rate: Bounded::new(Udec128::new_bps(40)).unwrap(),
        }
    }
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub bitcoin: Addr,
    pub dex: Addr,
    pub gateway: Addr,
    pub hyperlane: Hyperlane<Addr>,
    pub lending: Addr,
    pub oracle: Addr,
    pub taxman: Addr,
    pub warp: Addr,
}

// Default implementation that can be used in tests when the addresses are not
// needed.
impl Default for AppAddresses {
    fn default() -> Self {
        AppAddresses {
            account_factory: Addr::mock(0),
            bitcoin: Addr::mock(0),
            dex: Addr::mock(0),
            gateway: Addr::mock(0),
            hyperlane: Hyperlane::default(),
            lending: Addr::mock(0),
            oracle: Addr::mock(0),
            taxman: Addr::mock(0),
            warp: Addr::mock(0),
        }
    }
}

#[grug::derive(Serde)]
#[derive(Copy)]
pub struct Hyperlane<T> {
    pub ism: T,
    pub mailbox: T,
    pub va: T,
}

impl Default for Hyperlane<Addr> {
    fn default() -> Self {
        Hyperlane {
            ism: Addr::mock(0),
            mailbox: Addr::mock(0),
            va: Addr::mock(0),
        }
    }
}
