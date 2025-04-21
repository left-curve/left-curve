use {
    dango_types::lending::{Market, SECONDS_PER_YEAR},
    grug::{IsZero, MultiplyFraction, Number, NumberConst, QuerierWrapper, Timestamp, Udec128},
};

/// Update the state of a `Market` to account for accrued interests and protocol
/// fees since the last update.
pub fn update_indices(
    market: Market,
    querier: QuerierWrapper,
    current_time: Timestamp,
) -> anyhow::Result<Market> {
    debug_assert!(
        current_time >= market.last_update_time,
        "last update time is in the future! current time: {:?}, last update time: {:?}",
        current_time,
        market.last_update_time
    );

    // If there is no supply or borrow or last update time is equal to the
    // current time, then there is no interest to accrue
    if market.total_supplied(querier)?.is_zero()
        || market.total_borrowed_scaled.is_zero()
        || current_time == market.last_update_time
    {
        return Ok(market.set_last_update_time(current_time));
    }

    // Calculate interest rates
    let utilization_rate = market.utilization_rate(querier)?;
    let (borrow_rate, supply_rate) = market.interest_rate_model.calculate_rates(utilization_rate);

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
    let previous_total_borrowed = market.total_borrowed()?;
    let new_market = market.set_borrow_index(borrow_index);
    let new_total_borrowed = new_market.total_borrowed()?;
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
