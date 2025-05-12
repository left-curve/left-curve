use grug::{Bounded, Denom, Int128, Timestamp, Udec128, Uint128, ZeroInclusiveOneExclusive};

/// The state of the perps vault
#[grug::derive(Serde, Borsh)]
pub struct PerpsVaultState {
    /// The denom that is deposited into the vault.
    pub denom: Denom,
    /// The amount of the denom that is deposited into the vault.
    pub deposits: Uint128,
    /// The amount of shares that that have been minted.
    pub shares: Uint128,
}

/// Current state of a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketState {
    /// The denom of the market.
    pub denom: Denom,
    /// The long open interest of the market.
    pub long_oi: Uint128,
    /// The short open interest of the market.
    pub short_oi: Uint128,
    /// The last time the market was updated.
    pub last_updated: Timestamp,
}

/// Parameters for a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketParams {
    /// The denom of the market.
    pub denom: Denom,
    /// Whether trading is enabled for the market.
    pub trading_enabled: bool,
    /// The maximum long open interest of the market.
    pub max_long_oi: Uint128,
    /// The maximum short open interest of the market.
    pub max_short_oi: Uint128,
    /// The fee for opening a position in the market.
    pub position_open_fee: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The fee for closing a position in the market.
    pub position_close_fee: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The minimum size of a position.
    pub min_position_size: Uint128,
    /// Determines the funding rate for a given level of skew.
    /// The lower the `skew_scale` the higher the funding rate.
    pub skew_scale: Uint128,
    /// How fast the funding rate can change. See:
    /// <https://docs.synthetix.io/exchange/perps-basics/funding/technical-details>
    pub max_funding_velocity: Udec128,
}

/// The state of a perps position.
#[grug::derive(Serde, Borsh)]
pub struct PerpsPosition {
    /// The denom of the position.
    pub denom: Denom,
    /// The size of the position.
    pub size: Int128,
    /// The entry price of the position.
    pub entry_price: Udec128,
    /// The entry execution price of the position.
    pub entry_execution_price: Udec128,
    /// The skew at the time of entry.
    pub entry_skew: Udec128,
    /// The realized pnl of the position.
    pub realized_pnl: Int128,
}
