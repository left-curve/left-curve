use {
    dango_types::{
        Days,
        perps::{FundingRate, FundingVelocity, PairParam, PairState},
    },
    grug::MathResult,
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{HumanAmount, Ratio},
        grug::Duration,
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
}
