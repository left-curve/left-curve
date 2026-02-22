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
    pair_param: &PairParam,
) -> MathResult<FundingVelocity> {
    pair_state
        .skew
        .checked_div(pair_param.skew_scale)?
        .checked_mul3(pair_param.max_funding_velocity)
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
fn compute_current_funding_rate(
    pair_state: &PairState,
    pair_param: &PairParam,
    elapsed_time: Days,
) -> MathResult<FundingRate> {
    // Compute the funding rate velocity based on the current skew.
    let velocity = compute_funding_velocity(pair_state, pair_param)?;

    // Compute the funding rate based on the above two values, and clamp it to
    // between [-max_abs_funding_rate, max_abs_funding_rate].
    Ok(velocity
        .checked_mul4(elapsed_time)?
        .checked_add(pair_state.funding_rate)?
        .clamp(
            -pair_param.max_abs_funding_rate,
            pair_param.max_abs_funding_rate,
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
pub(super) fn compute_unrecorded_funding_per_unit(
    pair_state: &PairState,
    pair_param: &PairParam,
    current_time: Timestamp,
    oracle_price: UsdPrice,
) -> MathResult<(Ratio<UsdValue, HumanAmount>, FundingRate)> {
    // Compute the number of days elapsed since the last funding accrual.
    let elapsed_time = Days::try_from(current_time - pair_state.last_funding_time)?;

    // Compute the current funding rate based on last funding rate.
    let current_rate = compute_current_funding_rate(pair_state, pair_param, elapsed_time)?;

    // Compute the average funding rate bewtween the last accrual and now.
    let avg_rate = pair_state
        .funding_rate
        .checked_add(current_rate)?
        .checked_mul2(Ratio::<Dimensionless>::HALF)?;

    // Compute the unrecorded funding by "integrating" funding rate over the elapsed time.
    let unrecorded = avg_rate
        .checked_mul4(elapsed_time)?
        .checked_mul(oracle_price)?;

    Ok((unrecorded, current_rate))
}

/// Accrue funding for a pair. Update the accumulator, current rate, and timestamp.
///
/// ## Important
///
/// MUST be called before any OI-changing operation (`execute_fill`, liquidation)
/// to ensure correct accounting.
pub fn accrue_funding(
    pair_state: &mut PairState,
    pair_param: &PairParam,
    current_time: Timestamp,
    oracle_price: UsdPrice,
) -> MathResult<()> {
    // If no time has elapsed since the last update, nothing to do.
    if current_time == pair_state.last_funding_time {
        return Ok(());
    }

    let (unrecorded, current_rate) =
        compute_unrecorded_funding_per_unit(pair_state, pair_param, current_time, oracle_price)?;

    pair_state.funding_rate = current_rate;
    pair_state.last_funding_time = current_time;
    pair_state.funding_per_unit.checked_add_assign(unrecorded)?;

    Ok(())
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
        let pair_param = PairParam {
            skew_scale: Ratio::new_int(skew_scale),
            max_funding_velocity: Ratio::new_raw(max_funding_velocity_raw),
            ..Default::default()
        };

        assert_eq!(
            compute_funding_velocity(&pair_state, &pair_param).unwrap(),
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
        let pair_param = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };

        let elapsed_days = Days::try_from(Duration::from_seconds(elapsed_seconds)).unwrap();

        assert_eq!(
            compute_current_funding_rate(&pair_state, &pair_param, elapsed_days,).unwrap(),
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
        let pair_param = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };
        let oracle_price = Ratio::new_raw(oracle_price_raw);

        let (unrecorded, rate) = compute_unrecorded_funding_per_unit(
            &pair_state,
            &pair_param,
            current_time,
            oracle_price,
        )
        .unwrap();

        assert_eq!(unrecorded, Ratio::new_raw(expected_unrecorded_raw));
        assert_eq!(rate, Ratio::new_raw(expected_rate_raw));
    }

    // ---- accrue_funding tests ----

    #[test]
    fn accrue_funding_works() {
        let baseline = Timestamp::from_seconds(1_000_000);
        let one_day = Duration::from_seconds(86400);
        let oracle_price: UsdPrice = Ratio::new_raw(100_000_000); // 100 USD

        let pair_param = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };

        let mut pair_state = PairState {
            skew: HumanAmount::new(1000),
            funding_rate: Ratio::new_raw(0),
            last_funding_time: baseline,
            funding_per_unit: Ratio::new_raw(0),
            ..Default::default()
        };

        // 1) No-op when current_time == last_funding_time.
        let snapshot = pair_state.clone();
        accrue_funding(&mut pair_state, &pair_param, baseline, oracle_price).unwrap();
        assert_eq!(pair_state.funding_rate, snapshot.funding_rate);
        assert_eq!(pair_state.last_funding_time, snapshot.last_funding_time);
        assert_eq!(pair_state.funding_per_unit, snapshot.funding_per_unit,);

        // 2) Single accrual: 1 day, skew=1000, rate starts at 0, oracle=100.
        //    velocity = (1000/1000)*0.1 = 0.1 → current_rate = clamp(0 + 0.1*1, -0.05, 0.05) = 0.05
        //    avg_rate = (0 + 0.05)/2 = 0.025
        //    unrecorded = 0.025 * 1 * 100 = 2.5
        let t1 = baseline + one_day;
        accrue_funding(&mut pair_state, &pair_param, t1, oracle_price).unwrap();

        assert_eq!(pair_state.funding_rate, Ratio::new_raw(50_000));
        assert_eq!(pair_state.last_funding_time, t1);
        assert_eq!(pair_state.funding_per_unit, Ratio::new_raw(2_500_000),);

        // 3) Second accrual: another day, same skew. Rate already at max (0.05).
        //    velocity = 0.1 → current_rate = clamp(0.05 + 0.1*1, ...) = 0.05
        //    avg_rate = (0.05 + 0.05)/2 = 0.05
        //    unrecorded = 0.05 * 1 * 100 = 5.0
        //    cumulative = 2.5 + 5.0 = 7.5
        let t2 = t1 + one_day;
        accrue_funding(&mut pair_state, &pair_param, t2, oracle_price).unwrap();

        assert_eq!(pair_state.funding_rate, Ratio::new_raw(50_000));
        assert_eq!(pair_state.last_funding_time, t2);
        assert_eq!(pair_state.funding_per_unit, Ratio::new_raw(7_500_000),);
    }
}
