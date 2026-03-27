use {
    crate::{COMMISSION_RATE_OVERRIDES, USER_REFERRAL_DATA},
    dango_types::{
        UsdValue,
        account_factory::UserIndex,
        perps::{CommissionRate, FeeShareRatio, ReferralParam, Referrer, UserReferralData},
    },
    grug::{Bound, Duration, Number, Order as IterationOrder, StdResult, Storage, Timestamp},
    std::collections::BTreeMap,
};

/// Maximum fee share ratio a referrer can set.
pub(super) const MAX_FEE_SHARE_RATIO: FeeShareRatio = FeeShareRatio::new_percent(50);

/// Number of days in the rolling window for referees volume tiers.
pub(super) const COMMISSION_LOOKBACK_DAYS: u128 = 30;

/// Determine the commission rate for a referrer based on their
/// direct referees' 30-day rolling trading volume against configurable tiers.
/// N.B. This function assumes the user is a valid referrer (has set a fee share ratio).
pub fn calculate_commission_rate(
    storage: &dyn Storage,
    referrer: Referrer,
    block_timestamp: Timestamp,
    referral_param: &ReferralParam,
) -> StdResult<CommissionRate> {
    // If the referrer has a custom override, use it directly.
    if let Some(override_rate) = COMMISSION_RATE_OVERRIDES.may_load(storage, referrer)? {
        return Ok(override_rate);
    }

    let today = crate::round_to_day(block_timestamp);
    let lookback_start = today.saturating_sub(Duration::from_days(COMMISSION_LOOKBACK_DAYS));

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
    Ok(resolve_tiered_rate(
        referral_param.commission_rate_default,
        &referral_param.commission_rates_by_volume,
        window_referees_volume,
    ))
}

/// Resolve the highest qualifying rate from a tier map.
fn resolve_tiered_rate(
    default: CommissionRate,
    tiers: &BTreeMap<UsdValue, CommissionRate>,
    volume: UsdValue,
) -> CommissionRate {
    tiers
        .range(..=volume)
        .next_back()
        .map(|(_, &rate)| rate)
        .unwrap_or(default)
}

/// Load the cumulative referral data for a user with an optional upperbound.
/// If not specified, return the latest data.
pub(super) fn load_referral_data(
    storage: &dyn Storage,
    user_index: UserIndex,
    upper_bound: Option<Timestamp>,
) -> StdResult<UserReferralData> {
    let upper = upper_bound.map(Bound::Inclusive);

    USER_REFERRAL_DATA
        .prefix(user_index)
        .range(storage, None, upper, IterationOrder::Descending)
        .next()
        .transpose()
        .map(|opt| opt.map(|(_, data)| data).unwrap_or_default())
}
