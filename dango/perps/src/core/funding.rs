use {
    dango_types::{
        Ratio, UsdValue,
        perps::{PairParam, PairState},
    },
    grug::{Duration, MathResult},
};

/// Compute the current funding velocity (rate of change of the funding rate).
///
/// ```plain
/// velocity = (skew / skew_scale) * max_funding_velocity
/// ```
///
/// The velocity has the same sign as the skew:
///
/// - Positive skew (net long) → positive velocity → rate increases → longs pay more
/// - Negative skew (net short) → negative velocity → rate decreases → shorts pay more
/// - Zero skew → zero velocity → rate stays constant (drifts toward 0 naturally
///   only when the rate overshoots past zero)
pub fn compute_funding_velocity(
    pair_state: &PairState,
    pair_params: &PairParam,
) -> MathResult<Ratio<Ratio<UsdValue, Duration>, Duration>> {
    pair_state
        .skew
        .checked_div(pair_params.skew_scale)?
        .checked_mul2(pair_params.max_funding_velocity)
}
