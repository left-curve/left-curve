use grug::{Bounded, Denom, Udec128, Uint128, ZeroInclusiveOneExclusive};

/// Parameters for a perps market.
#[grug::derive(Serde, Borsh)]
pub struct PerpsMarketParams {
    /// The denom of the market.
    pub denom: Denom,
    /// Whether trading is enabled for the market.
    pub trading_enabled: bool,
    /// The maximum long open interest of the market, denominated in USD.
    pub max_long_oi: Uint128,
    /// The maximum short open interest of the market, denominated in USD.
    pub max_short_oi: Uint128,
    /// The fee for opening or increasing a position that is used if the position reduces the skew.
    pub maker_fee: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The fee for opening or increasing a position that is used if the position increases the skew.
    pub taker_fee: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    /// The minimum size of a position. Denominated in USD.
    pub min_position_size: Uint128,
    /// Determines the funding rate for a given level of skew.
    /// The lower the `skew_scale` the higher the funding rate.
    pub skew_scale: Uint128,
    /// How fast the funding rate can change. See:
    /// <https://docs.synthetix.io/exchange/perps-basics/funding/technical-details>
    pub max_funding_velocity: Udec128,
}
