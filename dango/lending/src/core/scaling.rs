//! Logics for the conversion between _underlying_ and _scaled_ asset amounts.
//!
//! In the Dango lending protocol, we use a decimal variable termed _index_ to
//! account for the continuous accrual of interest over time.
//!
//! For example, inside the contract we stored a _scaled depsoit amount_ of 100.
//! If the `supply_index` is 1.2, this means there is `100 * 1.2 = 120` units of
//! tokens supplied.
//!
//! Over time, as interest accrues, the index increases, but the _scaled_ amount
//! remains the same. As such, the _underlying_ amount increases.
//!
//! ## On rounding errors
//!
//! Sometimes the underlying amount may not be a full integer number. For example,
//! if the index is 1.234, the underlying amount would be `100 * 1.234 = 123.4`.
//! Do we round this up to 124, or down to 123?
//!
//! Incorrect rounding is one of the most exploited vulnerabilities in lending
//! markets. See:
//!
//! - <https://www.dlnews.com/articles/defi/hackers-continue-to-profit-from-defi-developers-math-problem/>
//! - <https://osec.io/blog/2024-01-18-rounding-bugs>
//!
//! In Dango, we follow this principle: **always round to the advantage to the
//! protocol, and to the disadvantage of the user**.
//!
//! This means always round _down_ the amount that the protocol owes the user,
//! and _up_ the amount the user owes the protocol.
//!
//! In this file, we provide four functions for the conversion between scaled
//! and underlying amounts. They should be considered the _source of truth for
//! such conversions_. All other codes that perform such conversions should call
//! these functions.
use {
    dango_types::lending::Market,
    grug::{MathResult, MultiplyFraction, Uint128},
};

/// Convert an underlying (unscaled) amount of deposit to a scaled amount.
///
/// NOTE: round down.
pub fn underlying_asset_to_scaled(underlying: Uint128, market: &Market) -> MathResult<Uint128> {
    underlying.checked_div_dec_floor(market.supply_index)
}

/// Convert a scaled amount of deposit to an underlying (unscaled) amount.
///
/// NOTE: round down.
pub fn scaled_asset_to_underlying(scaled: Uint128, market: &Market) -> MathResult<Uint128> {
    scaled.checked_mul_dec_floor(market.supply_index)
}

/// Convert an underlying (unscaled) amount of debt to a scaled amount.
///
/// NOTE: round up.
pub fn underlying_debt_to_scaled(underlying: Uint128, market: &Market) -> MathResult<Uint128> {
    underlying.checked_div_dec_ceil(market.borrow_index)
}

/// Convert a scaled amount of debt to an underlying (unscaled) amount.
///
/// NOTE: round up.
pub fn scaled_debt_to_underlying(scaled: Uint128, market: &Market) -> MathResult<Uint128> {
    scaled.checked_mul_dec_ceil(market.borrow_index)
}
