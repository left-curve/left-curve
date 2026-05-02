use {
    crate::{referral::load_referral_data, state::COMMISSION_RATE_OVERRIDES, volume::round_to_day},
    dango_order_book::UsdValue,
    dango_types::perps::{CommissionRate, Param, Referrer},
    grug::{Duration, Number, StdResult, Storage, Timestamp},
};

/// Number of days in the rolling window for referees volume tiers.
const COMMISSION_LOOKBACK_DAYS: Duration = Duration::from_days(30);

/// Determine the commission rate for a referrer based on their
/// direct referees' 30-day rolling trading volume against configurable tiers.
/// N.B. This function assumes the user is a valid referrer (has set a fee share ratio).
pub fn calculate_commission_rate(
    storage: &dyn Storage,
    referrer: Referrer,
    block_timestamp: Timestamp,
    param: &Param,
) -> StdResult<CommissionRate> {
    // If the referrer has a custom override, use it directly.
    if let Some(override_rate) = COMMISSION_RATE_OVERRIDES.may_load(storage, referrer)? {
        return Ok(override_rate);
    }

    let today = round_to_day(block_timestamp);
    let lookback_start = today.saturating_sub(COMMISSION_LOOKBACK_DAYS);

    // Load the latest cumulative data for the referrer.
    let latest = load_referral_data(storage, referrer, None)?;

    // Load the cumulative data at the start of the window.
    let start = load_referral_data(storage, referrer, Some(lookback_start))?;

    // 30-day referees volume = current cumulative - start cumulative.
    let window_referees_volume = latest
        .referees_volume
        .checked_sub(start.referees_volume)
        .unwrap_or(UsdValue::ZERO);

    // Find the highest qualifying tier.
    Ok(param
        .referrer_commission_rates
        .resolve(window_referees_volume))
}
