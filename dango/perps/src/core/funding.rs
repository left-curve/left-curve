use {
    dango_types::{Days, Dimensionless, FundingPerUnit, FundingRate, Quantity, UsdPrice, UsdValue},
    grug::{MathResult, StdResult},
};

/// Walk an ordered sequence of `(limit_price, size)` pairs and compute the
/// volume-weighted average execution price for filling `impact_size` worth
/// of notional value.
///
/// Each item is `(limit_price, absolute_order_size)`. The caller is responsible
/// for iterating the correct side of the book (bids or asks) in price-priority
/// order and mapping storage entries to `(real_price, absolute_size)`.
///
/// Returns: `Some(vwap)` if enough depth exists, `None` otherwise.
pub fn compute_impact_price(
    orders: impl Iterator<Item = StdResult<(UsdPrice, Quantity)>>,
    impact_size: UsdValue,
) -> StdResult<Option<UsdPrice>> {
    let mut total_size = Quantity::ZERO;
    let mut total_notional = UsdValue::ZERO;

    for result in orders {
        let (price, size) = result?;
        let order_notional = size.checked_mul(price)?;
        let remaining = impact_size.checked_sub(total_notional)?;

        if order_notional >= remaining {
            // Partial fill of this order completes the impact notional.
            let partial_size = remaining.checked_div(price)?;

            total_size.checked_add_assign(partial_size)?;
            total_notional = impact_size;

            break;
        }

        total_size.checked_add_assign(size)?;
        total_notional.checked_add_assign(order_notional)?;
    }

    if total_notional < impact_size {
        return Ok(None);
    }

    Ok(Some(total_notional.checked_div(total_size)?))
}

/// Compute the premium from impact bid/ask prices relative to the oracle price.
///
/// Formula:
/// ```text
/// premium = [max(0, impact_bid - oracle) - max(0, oracle - impact_ask)] / oracle
/// ```
///
/// If a side is `None`, its `max(0, ...)` term is zero.
///
/// Returns: premium as a `Dimensionless` value.
pub fn compute_premium(
    impact_bid: Option<UsdPrice>,
    impact_ask: Option<UsdPrice>,
    oracle_price: UsdPrice,
) -> MathResult<Dimensionless> {
    let bid_term = match impact_bid {
        Some(bid) if bid > oracle_price => bid.checked_sub(oracle_price)?,
        _ => UsdPrice::ZERO,
    };

    let ask_term = match impact_ask {
        Some(ask) if ask < oracle_price => oracle_price.checked_sub(ask)?,
        _ => UsdPrice::ZERO,
    };

    bid_term.checked_sub(ask_term)?.checked_div(oracle_price)
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
        dango_types::{Days, Dimensionless, FundingPerUnit, FundingRate, UsdPrice, UsdValue},
        grug::{Duration, StdResult},
        test_case::test_case,
    };

    // ---- compute_impact_price tests ----

    #[test]
    fn impact_price_empty_book() {
        let orders = std::iter::empty::<StdResult<(UsdPrice, Quantity)>>();
        let result = compute_impact_price(orders, UsdValue::new_int(10_000)).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn impact_price_insufficient_depth() {
        let orders = vec![Ok((UsdPrice::new_int(50_000), Quantity::new_int(1)))];
        // Need 100_000 notional but only have 50_000
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(100_000)).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn impact_price_exact_fill_single_order() {
        // Single order: price=50_000, size=2 → notional=100_000
        let orders = vec![Ok((UsdPrice::new_int(50_000), Quantity::new_int(2)))];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(100_000)).unwrap();
        assert_eq!(result, Some(UsdPrice::new_int(50_000)));
    }

    #[test]
    fn impact_price_partial_fill_last_order() {
        // Two orders: price=100, size=5 (notional=500) and price=110, size=10 (notional=1100)
        // Impact notional = 1000 → fill all of first (500) then 500/110 ≈ 4.545454 of second
        // total_size = 5 + 500/110 = 5 + 4.545454... = 9.545454...
        // VWAP = 1000 / 9.545454... = 104.761904...
        let orders = vec![
            Ok((UsdPrice::new_int(100), Quantity::new_int(5))),
            Ok((UsdPrice::new_int(110), Quantity::new_int(10))),
        ];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(1_000)).unwrap();
        let vwap = result.unwrap();
        // VWAP should be between 100 and 110
        assert!(vwap > UsdPrice::new_int(100));
        assert!(vwap < UsdPrice::new_int(110));
    }

    #[test]
    fn impact_price_multi_order_exact() {
        // Two orders that exactly fill: price=100, size=5 (500) + price=200, size=5 (1000)
        // total = 1500, need 1500, total_size=10 → VWAP = 1500/10 = 150
        let orders = vec![
            Ok((UsdPrice::new_int(100), Quantity::new_int(5))),
            Ok((UsdPrice::new_int(200), Quantity::new_int(5))),
        ];
        let result = compute_impact_price(orders.into_iter(), UsdValue::new_int(1_500)).unwrap();
        assert_eq!(result, Some(UsdPrice::new_int(150)));
    }

    // ---- compute_premium tests ----

    #[test_case(Some(101), Some(99),  100,       0 ; "symmetric around oracle")]
    #[test_case(Some(103), Some(99),  100,  20_000 ; "bid skewed above oracle")]
    #[test_case(Some(99),  Some(97),  100, -30_000 ; "ask skewed below oracle")]
    #[test_case(Some(102), None,      100,  20_000 ; "bid only")]
    #[test_case(None,      Some(98),  100, -20_000 ; "ask only")]
    #[test_case(None,      None,      100,       0 ; "both none")]
    #[test_case(Some(99),  Some(101), 100,       0 ; "bid below ask above oracle")]
    #[test_case(Some(100), Some(100), 100,       0 ; "bid and ask at oracle")]
    fn compute_premium_works(
        bid: Option<i128>,
        ask: Option<i128>,
        oracle: i128,
        expected_raw: i128,
    ) {
        let impact_bid = bid.map(UsdPrice::new_int);
        let impact_ask = ask.map(UsdPrice::new_int);
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
