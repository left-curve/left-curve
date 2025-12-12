use {
    crate::constants::usdc,
    grug::{Addr, Bounded, Coins, Udec128, ZeroInclusiveOneExclusive, coins},
};

/// Application-specific configurations of the Dango chain.
#[grug::derive(Serde)]
pub struct AppConfig {
    /// Addresses of relevant Dango contracts.
    pub addresses: AppAddresses,
    /// The minimum deposit required to onboard a user.
    pub minimum_deposit: Coins,
    /// The maker fee for the DEX.
    pub maker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The taker fee for the DEX.
    pub taker_fee_rate: Bounded<Udec128, ZeroInclusiveOneExclusive>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            addresses: Default::default(),
            minimum_deposit: coins! { usdc::DENOM.clone() => 10_000_000 }, // 10 USDC
            maker_fee_rate: Bounded::new(Udec128::new_bps(25)).unwrap(),
            taker_fee_rate: Bounded::new(Udec128::new_bps(40)).unwrap(),
        }
    }
}

/// Addresses of relevant Dango contracts.
#[grug::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub dex: Addr,
    pub gateway: Addr,
    pub hyperlane: Hyperlane<Addr>,
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
            dex: Addr::mock(0),
            gateway: Addr::mock(0),
            hyperlane: Hyperlane::default(),
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
