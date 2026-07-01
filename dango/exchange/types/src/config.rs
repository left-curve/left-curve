use {
    crate::constants::usdc,
    dango_primitives::{Addr, Coins, coins},
};

/// Application-specific configurations of the Dango chain.
#[dango_primitives::derive(Serde)]
pub struct AppConfig {
    /// Addresses of relevant Dango contracts.
    pub addresses: AppAddresses,
    /// The minimum deposit required to onboard a user.
    pub minimum_deposit: Coins,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            addresses: Default::default(),
            minimum_deposit: coins! { usdc::DENOM.clone() => 10_000_000 }, // 10 USDC
        }
    }
}

/// Addresses of relevant Dango contracts.
#[dango_primitives::derive(Serde)]
pub struct AppAddresses {
    pub account_factory: Addr,
    pub gateway: Addr,
    pub hyperlane: Hyperlane<Addr>,
    pub oracle: Addr,
    pub perps: Addr,
    pub warp: Addr,
}

// Default implementation that can be used in tests when the addresses are not
// needed.
impl Default for AppAddresses {
    fn default() -> Self {
        AppAddresses {
            account_factory: Addr::mock(0),
            gateway: Addr::mock(0),
            hyperlane: Hyperlane::default(),
            oracle: Addr::mock(0),
            perps: Addr::mock(0),
            warp: Addr::mock(0),
        }
    }
}

#[dango_primitives::derive(Serde)]
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
