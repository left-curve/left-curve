use {
    dango_types::{
        Days, Dimensionless, HumanAmount, Ratio, UsdPrice, UsdValue,
        perps::{FundingRate, FundingVelocity, PairParam, PairState},
    },
    grug::{MathResult, Timestamp},
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
fn compute_funding_velocity(
    pair_state: &PairState,
    pair_params: &PairParam,
) -> MathResult<FundingVelocity> {
    pair_state
        .skew
        .checked_div(pair_params.skew_scale)?
        .checked_mul2(pair_params.max_funding_velocity)
}

/// Compute the current funding rate, accounting for time elapsed since
/// the last accrual.
///
/// ```plain
/// current_rate = clamp(
///   last_rate + velocity * elapsed_days,
///   -max_abs_funding_rate,
///   max_abs_funding_rate
/// )
/// ```
///
/// The rate is clamped to prevent runaway funding that could cause cascading
/// liquidations and bad debt spirals during prolonged skew.
pub fn compute_current_funding_rate(
    pair_state: &PairState,
    pair_params: &PairParam,
    elapsed_time: Days,
) -> MathResult<FundingRate> {
    // Compute the funding rate velocity based on the current skew.
    let velocity = compute_funding_velocity(pair_state, pair_params)?;

    // Compute the funding rate based on the above two values, and clamp it to
    // between [-max_abs_funding_rate, max_abs_funding_rate].
    Ok(velocity
        .checked_mul3(elapsed_time)?
        .checked_add(pair_state.funding_rate)?
        .clamp(
            -pair_params.max_abs_funding_rate,
            pair_params.max_abs_funding_rate,
        ))
}

/// Compute the funding per unit of position size that has accrued since
/// the last accrual but not yet been recorded, along with the current
/// funding rate.
///
/// Returns `(unrecorded_funding_per_unit, current_rate)`.
///
/// Between two timestamps t1 < t2, the accumulated funding per unit is:
///
/// ```plain
/// unrecorded := ∫ r(t) p(t) dt
/// ```
///
/// where `r(t)` is the funding rate at time `t`, and `p(t)` is the oracle price
/// at time `t`.
///
/// - We assume `p(t)` is a constant `p` between the two timestamps.
///   This is a reasonable assumption, considering in Dango, we trigger a
///   funding rate accrual each block right after the oracle price update.
/// - We know `r(t)` evolves linearly over time. Therefore, the integral
///   `∫ r(t) dt` simply equals the average rate multiplies the duration.
///
/// The equation simplifies to:
///
/// ```plain
/// unrecorded := p * ∫ r(t) dt = p * (1/2) * (v(t1) + v(t2)) * (t1 - t2)
/// ```
///
/// Or in pseudocode:
///
/// ```plain
/// avg_rate = (previous_rate + current_rate) / 2
/// unrecorded = avg_rate * elapsed_days * oracle_price
/// ```
pub fn compute_unrecorded_funding_per_unit(
    pair_state: &PairState,
    pair_params: &PairParam,
    current_time: Timestamp,
    oracle_price: UsdPrice,
) -> MathResult<(Ratio<UsdValue, HumanAmount>, FundingRate)> {
    // Compute the number of days elapsed since the last funding accrual.
    let elapsed_time = Days::try_from(current_time - pair_state.last_funding_time)?;

    // Compute the current funding rate based on last funding rate.
    let current_rate = compute_current_funding_rate(pair_state, pair_params, elapsed_time)?;

    // Compute the average funding rate bewtween the last accrual and now.
    let avg_rate = pair_state
        .funding_rate
        .checked_add(current_rate)?
        .checked_mul4(Ratio::<Dimensionless>::HALF)?;

    // Compute the unrecorded funding by "integrating" funding rate over the elapsed time.
    let unrecorded = avg_rate
        .checked_mul3(elapsed_time)?
        .checked_mul(oracle_price)?;

    Ok((unrecorded, current_rate))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{HumanAmount, Ratio},
        grug::{Duration, Timestamp},
        test_case::test_case,
    };

    // ---- compute_funding_velocity tests ----

    // velocity = (skew / skew_scale) * max_funding_velocity
    //
    // Using skew_scale = 1000, max_funding_velocity = 0.1/day² (100_000 raw):
    //   half:  (500 / 1000)  * 0.1  =  0.05  (50_000 raw)
    //   full:  (1000 / 1000) * 0.1  =  0.1   (100_000 raw)
    //   neg:   (-1000 / 1000) * 0.1 = -0.1   (-100_000 raw)
    #[test_case(    0, 1000, 100_000,         0 ; "zero skew")]
    #[test_case(  500, 1000, 100_000,    50_000 ; "half scale positive skew")]
    #[test_case( 1000, 1000, 100_000,   100_000 ; "full scale positive skew")]
    #[test_case(-1000, 1000, 100_000,  -100_000 ; "full scale negative skew")]
    fn compute_funding_velocity_works(
        skew: i128,
        skew_scale: i128,
        max_funding_velocity_raw: i128,
        expected_raw: i128,
    ) {
        let pair_state = PairState {
            skew: HumanAmount::new(skew),
            ..Default::default()
        };
        let pair_params = PairParam {
            skew_scale: Ratio::new_int(skew_scale),
            max_funding_velocity: Ratio::new_raw(max_funding_velocity_raw),
            ..Default::default()
        };

        assert_eq!(
            compute_funding_velocity(&pair_state, &pair_params).unwrap(),
            Ratio::new_raw(expected_raw),
        );
    }

    // ---- compute_current_funding_rate tests ----

    // current_rate = clamp(last_rate + velocity * elapsed, -max, max)
    //
    // Fixed params:
    //   skew_scale             = 1000
    //   max_funding_velocity   = 0.1/day²  (100_000 raw)
    //   max_abs_funding_rate   = 0.05/day   (50_000 raw)
    #[test_case(     0,       0,     0,       0 ; "zero elapsed zero skew")]
    #[test_case(  1000,       0, 86400,  50_000 ; "1 day full skew clamped to max")]
    #[test_case(   500,       0, 86400,  50_000 ; "1 day half skew reaches max exactly")]
    #[test_case( -1000,       0, 86400, -50_000 ; "1 day negative full skew clamped to min")]
    #[test_case(  1000,       0, 43200,  50_000 ; "half day full skew reaches max exactly")]
    #[test_case(     0,  30_000, 86400,  30_000 ; "existing rate preserved with zero skew")]
    #[test_case(  1000,  40_000, 86400,  50_000 ; "would exceed max clamped to upper bound")]
    #[test_case( -1000, -40_000, 86400, -50_000 ; "would exceed min clamped to lower bound")]
    fn compute_current_funding_rate_works(
        skew: i128,
        last_rate_raw: i128,
        elapsed_seconds: u128,
        expected_raw: i128,
    ) {
        let pair_state = PairState {
            skew: HumanAmount::new(skew),
            funding_rate: Ratio::new_raw(last_rate_raw),
            ..Default::default()
        };
        let pair_params = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };

        let elapsed_days = Days::try_from(Duration::from_seconds(elapsed_seconds)).unwrap();

        assert_eq!(
            compute_current_funding_rate(&pair_state, &pair_params, elapsed_days,).unwrap(),
            Ratio::new_raw(expected_raw),
        );
    }

    // ---- compute_unrecorded_funding_per_unit tests ----

    // unrecorded = avg_rate * elapsed_days * oracle_price
    // avg_rate   = (last_rate + current_rate) / 2
    //
    // Fixed params (same as above):
    //   skew_scale             = 1000
    //   max_funding_velocity   = 0.1/day²  (100_000 raw)
    //   max_abs_funding_rate   = 0.05/day   (50_000 raw)
    //
    // Timestamp baseline: last_funding_time = 1_000_000s, current_time = baseline + elapsed.
    #[test_case(     0,       0,     0, 100_000_000,         0,      0 ; "zero elapsed")]
    #[test_case(  1000,       0, 86400, 100_000_000, 2_500_000, 50_000 ; "full positive skew 1 day")]
    #[test_case( -1000,       0, 86400, 100_000_000,-2_500_000,-50_000 ; "full negative skew 1 day")]
    #[test_case(     0,  30_000, 86400, 100_000_000, 3_000_000, 30_000 ; "zero skew existing rate")]
    #[test_case(   500,       0, 43200, 100_000_000,   625_000, 25_000 ; "half day half skew")]
    #[test_case(  1000,       0, 86400,  50_000_000, 1_250_000, 50_000 ; "different oracle price")]
    #[test_case(  1000,  50_000, 86400, 100_000_000, 5_000_000, 50_000 ; "rate already at max")]
    fn compute_unrecorded_funding_per_unit_works(
        skew: i128,
        last_rate_raw: i128,
        elapsed_seconds: u128,
        oracle_price_raw: i128,
        expected_unrecorded_raw: i128,
        expected_rate_raw: i128,
    ) {
        let baseline = Timestamp::from_seconds(1_000_000);
        let current_time = Timestamp::from_seconds(1_000_000 + elapsed_seconds);

        let pair_state = PairState {
            skew: HumanAmount::new(skew),
            funding_rate: Ratio::new_raw(last_rate_raw),
            last_funding_time: baseline,
            ..Default::default()
        };
        let pair_params = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };
        let oracle_price = Ratio::new_raw(oracle_price_raw);

        let (unrecorded, rate) =
            compute_unrecorded_funding_per_unit(&pair_state, &pair_params, current_time, oracle_price)
                .unwrap();

        assert_eq!(unrecorded, Ratio::new_raw(expected_unrecorded_raw));
        assert_eq!(rate, Ratio::new_raw(expected_rate_raw));
    }
}
