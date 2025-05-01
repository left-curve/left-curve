use {
    crate::{calculate_rates, core},
    dango_types::lending::{Market, SECONDS_PER_YEAR},
    grug::{
        Bounded, IsZero, MathResult, MultiplyFraction, Number, NumberConst, StdResult, Timestamp,
        Udec128, Uint128, ZeroInclusiveOneInclusive,
    },
};

/// Update the state of a `Market` to account for accrued interests and protocol
/// fees since the last update.
pub fn update_indices(market: Market, current_time: Timestamp) -> anyhow::Result<Market> {
    debug_assert!(
        current_time >= market.last_update_time,
        "last update time is in the future! current time: {:?}, last update time: {:?}",
        current_time,
        market.last_update_time
    );

    // If there is no supply or borrow or last update time is equal to the
    // current time, then there is no interest to accrue
    if total_supplied(&market)?.is_zero()
        || market.total_borrowed_scaled.is_zero()
        || current_time == market.last_update_time
    {
        return Ok(market.set_last_update_time(current_time));
    }

    // Calculate interest rates
    let utilization_rate = utilization_rate(&market)?;
    let (borrow_rate, supply_rate) = calculate_rates(&market.interest_rate_model, utilization_rate);

    // Update the indices
    let time_delta = current_time - market.last_update_time;
    let time_out_of_year =
        Udec128::checked_from_ratio(time_delta.into_seconds(), SECONDS_PER_YEAR)?;
    let borrow_index = market
        .borrow_index
        .checked_mul(Udec128::ONE.checked_add(borrow_rate.checked_mul(time_out_of_year)?)?)?;
    let supply_index = market
        .supply_index
        .checked_mul(Udec128::ONE.checked_add(supply_rate.checked_mul(time_out_of_year)?)?)?;

    // Calculate the protocol fee
    let previous_total_borrowed = total_borrowed(&market)?;
    let new_market = market.set_borrow_index(borrow_index);
    let new_total_borrowed = total_borrowed(&new_market)?;
    let borrow_interest = new_total_borrowed.checked_sub(previous_total_borrowed)?;
    let protocol_fee =
        borrow_interest.checked_mul_dec(*new_market.interest_rate_model.reserve_factor)?;
    let protocol_fee_scaled = protocol_fee.checked_div_dec_floor(supply_index)?;

    // Return the new market state
    Ok(new_market
        .set_supply_index(supply_index)
        .set_last_update_time(current_time)
        .add_pending_protocol_fee(protocol_fee_scaled)?)
}

/// Compute the `Market`'s utilization rate.
pub fn utilization_rate(
    market: &Market,
) -> anyhow::Result<Bounded<Udec128, ZeroInclusiveOneInclusive>> {
    let total_borrowed = total_borrowed(market)?;
    let total_supplied = total_supplied(market)?;

    if total_supplied.is_zero() {
        return Ok(Bounded::new_unchecked(Udec128::ZERO));
    }

    let utilization_rate = Udec128::checked_from_ratio(total_borrowed, total_supplied)?;

    // Limit utilization rate to 100%
    // This can happen if 100% of the supply is borrowed, which can then cause
    // borrowing to outgrow the supply due to interest accrual.
    if utilization_rate > Udec128::new_percent(100) {
        return Ok(Bounded::new_unchecked(Udec128::new_percent(100)));
    }

    Ok(Bounded::new_unchecked(utilization_rate))
}

/// Find the total amount of coins supplied to the `Market`.
pub fn total_supplied(market: &Market) -> StdResult<Uint128> {
    let amount_scaled = market.total_supplied_scaled;
    let amount_scaled = amount_scaled.checked_add(market.pending_protocol_fee_scaled)?;
    Ok(core::scaled_asset_to_underlying(amount_scaled, market)?)
}

/// Find the total amount of coins borrowed from the `Market`.
pub fn total_borrowed(market: &Market) -> MathResult<Uint128> {
    core::scaled_debt_to_underlying(market.total_borrowed_scaled, market)
}
