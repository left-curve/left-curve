use {
    dango_order_book::{Days, Dimensionless, FundingPerUnit, FundingRate, UsdPrice},
    grug_math::MathResult,
};

/// Compute the premium from the midpoint of the two impact prices relative
/// to the oracle price.
///
/// Formula:
/// ```text
/// mid     = (impact_bid + impact_ask) / 2
/// premium = (mid - oracle) / oracle
/// ```
///
/// The caller is responsible for passing non-missing impact prices: if either
/// side of the book is empty, the caller should skip the sample rather than
/// attempt to compute a one-sided mid.
///
/// Returns: premium as a `Dimensionless` value.
pub fn compute_premium(
    impact_bid: UsdPrice,
    impact_ask: UsdPrice,
    oracle_price: UsdPrice,
) -> MathResult<Dimensionless> {
    let mid = impact_bid.checked_add(impact_ask)?.half();
    mid.checked_sub(oracle_price)?.checked_div(oracle_price)
}

/// Compute the funding delta to apply to the `funding_per_unit` accumulator,
/// given the average premium over the sampling period.
///
/// The average premium is interpreted as a per-day funding rate, clamped to
/// `[-max_abs_funding_rate, +max_abs_funding_rate]`, then scaled by the actual
/// elapsed interval.
///
/// Returns: `(funding_delta, clamped_rate)` — the delta as `FundingPerUnit`
/// and the clamped per-day `FundingRate` that produced it.
pub fn compute_funding_delta(
    avg_premium: Dimensionless,
    oracle_price: UsdPrice,
    max_abs_funding_rate: FundingRate,
    interval: Days,
) -> MathResult<(FundingPerUnit, FundingRate)> {
    // Reinterpret the dimensionless average premium as a per-day funding rate.
    let rate_per_day = FundingRate::new(avg_premium.into_inner());

    // Clamp to the configured bounds.
    let clamped_rate = rate_per_day.clamp(-max_abs_funding_rate, max_abs_funding_rate);

    // funding_delta = clamped_rate * interval * oracle_price
    // FundingRate(day⁻¹) × Days(day) = Dimensionless
    // Dimensionless × UsdPrice(usd/qty) = FundingPerUnit(usd/qty)
    let delta = clamped_rate
        .checked_mul(interval)?
        .checked_mul(oracle_price)?;

    Ok((delta, clamped_rate))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{Days, Dimensionless, FundingPerUnit, FundingRate, UsdPrice},
        grug_types::Duration,
        test_case::test_case,
    };

    // ---- compute_premium tests ----

    #[test_case( 99, 101, 100,       0 ; "mid exactly at oracle")]
    #[test_case(101, 103, 100,  20_000 ; "mid above oracle (vault short skew)")]
    #[test_case( 97,  99, 100, -20_000 ; "mid below oracle (vault long skew)")]
    #[test_case(100, 100, 100,       0 ; "bid and ask at oracle")]
    #[test_case( 99, 103, 100,  10_000 ; "asymmetric spread, mid above oracle")]
    #[test_case( 99, 100, 100,  -5_000 ; "odd sum, mid below oracle by half unit")]
    fn compute_premium_works(bid: i128, ask: i128, oracle: i128, expected_raw: i128) {
        let impact_bid = UsdPrice::new_int(bid);
        let impact_ask = UsdPrice::new_int(ask);
        let oracle_price = UsdPrice::new_int(oracle);

        let premium = compute_premium(impact_bid, impact_ask, oracle_price).unwrap();
        assert_eq!(premium, Dimensionless::new_raw(expected_raw));
    }

    // ---- compute_funding_delta tests ----

    #[test]
    fn funding_delta_normal() {
        // avg_premium = 0.01 (1%), oracle = 100, max = 0.05/day, interval = 1 day
        // rate_per_day = 0.01, clamped = 0.01
        // delta = 0.01 * 1 * 100 = 1.0
        let avg_premium = Dimensionless::new_raw(10_000); // 0.01
        let oracle_price = UsdPrice::new_int(100);
        let max_rate = FundingRate::new_raw(50_000); // 0.05/day
        let interval = Days::from_duration(Duration::from_seconds(86400)).unwrap();

        let (delta, rate) =
            compute_funding_delta(avg_premium, oracle_price, max_rate, interval).unwrap();
        assert_eq!(delta, FundingPerUnit::new_raw(1_000_000)); // 1.0
        assert_eq!(rate, FundingRate::new_raw(10_000)); // 0.01/day (unclamped)
    }

    #[test]
    fn funding_delta_clamped() {
        // avg_premium = 0.10 (10%), but max = 0.05/day → clamped to 0.05
        // delta = 0.05 * 1 * 100 = 5.0
        let avg_premium = Dimensionless::new_raw(100_000); // 0.10
        let oracle_price = UsdPrice::new_int(100);
        let max_rate = FundingRate::new_raw(50_000); // 0.05/day
        let interval = Days::from_duration(Duration::from_seconds(86400)).unwrap();

        let (delta, rate) =
            compute_funding_delta(avg_premium, oracle_price, max_rate, interval).unwrap();
        assert_eq!(delta, FundingPerUnit::new_raw(5_000_000)); // 5.0
        assert_eq!(rate, max_rate); // clamped to max
    }

    #[test]
    fn funding_delta_negative_clamped() {
        // avg_premium = -0.10, max = 0.05/day → clamped to -0.05
        // delta = -0.05 * 1 * 100 = -5.0
        let avg_premium = Dimensionless::new_raw(-100_000); // -0.10
        let oracle_price = UsdPrice::new_int(100);
        let max_rate = FundingRate::new_raw(50_000); // 0.05/day
        let interval = Days::from_duration(Duration::from_seconds(86400)).unwrap();

        let (delta, rate) =
            compute_funding_delta(avg_premium, oracle_price, max_rate, interval).unwrap();
        assert_eq!(delta, FundingPerUnit::new_raw(-5_000_000)); // -5.0
        assert_eq!(rate, -max_rate); // clamped to -max
    }

    #[test]
    fn funding_delta_half_day() {
        // avg_premium = 0.02, oracle = 50_000, max = 0.05/day, interval = 0.5 day
        // rate_per_day = 0.02, clamped = 0.02
        // delta = 0.02 * 0.5 * 50_000 = 500
        let avg_premium = Dimensionless::new_raw(20_000); // 0.02
        let oracle_price = UsdPrice::new_int(50_000);
        let max_rate = FundingRate::new_raw(50_000); // 0.05/day
        let interval = Days::from_duration(Duration::from_seconds(43200)).unwrap(); // 12h

        let (delta, rate) =
            compute_funding_delta(avg_premium, oracle_price, max_rate, interval).unwrap();
        assert_eq!(delta, FundingPerUnit::new_int(500));
        assert_eq!(rate, FundingRate::new_raw(20_000)); // 0.02/day (unclamped)
    }
}
