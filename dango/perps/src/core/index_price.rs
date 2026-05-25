use {
    dango_order_book::{Dimensionless, UsdPrice},
    grug_math::MathResult,
    grug_types::Duration,
};

/// EWMA time constant (tau): 30 minutes. Controls how quickly the index price
/// converges toward the order book. After tau of sustained pressure, the index
/// closes ~63% of the gap.
pub const INDEX_TIME_CONSTANT: Duration = Duration::from_minutes(30);

/// Approximate `exp(-x)` via degree-4 Taylor polynomial.
///
/// Accurate to < 1e-7 for `x in [0, 0.1]` (the EWMA's operating range after
/// the max-tick-fraction cap).
fn exp_neg_approx(x: Dimensionless) -> MathResult<Dimensionless> {
    let x2 = x.checked_mul(x)?;
    let x3 = x2.checked_mul(x)?;
    let x4 = x3.checked_mul(x)?;

    Dimensionless::ONE
        .checked_sub(x)?
        .checked_add(x2.checked_div(Dimensionless::new_int(2))?)?
        .checked_sub(x3.checked_div(Dimensionless::new_int(6))?)?
        .checked_add(x4.checked_div(Dimensionless::new_int(24))?)
}

/// Compute the next EWMA index price given the current index price and the
/// order book's impact bid/ask levels.
///
/// Implements the trade.xyz closed-market oracle formula:
///
/// ```text
/// IPD   = max(bid - S, 0) - max(S - ask, 0)
/// Δt*   = min(Δt, c × τ)          where c = 0.1
/// β     = exp(-Δt* / τ)
/// S_new = S + (1 - β) × IPD
/// ```
///
/// If a side of the book has insufficient depth (passed as `None`), that side's
/// contribution to the IPD is zero. If both sides are `None`, the price does
/// not move.
pub fn compute_ewma_index_price(
    current_index: UsdPrice,
    impact_bid: Option<UsdPrice>,
    impact_ask: Option<UsdPrice>,
    delta_t: Duration,
) -> MathResult<UsdPrice> {
    let tau_millis = INDEX_TIME_CONSTANT.into_millis();
    let dt_millis = delta_t.into_millis();

    // Δt* = min(Δt, c × τ) where c = 0.1
    let c_tau_millis = tau_millis / 10;
    let dt_star_millis = dt_millis.min(c_tau_millis);

    if dt_star_millis == 0 {
        return Ok(current_index);
    }

    // x = Δt* / τ (dimensionless ratio, always in [0, 0.1])
    let x = Dimensionless::new_int(dt_star_millis as i128)
        .checked_div(Dimensionless::new_int(tau_millis as i128))?;

    // α = 1 - β = 1 - exp(-x)
    let alpha = Dimensionless::ONE.checked_sub(exp_neg_approx(x)?)?;

    // IPD = max(bid - S, 0) - max(S - ask, 0)
    let bid_contribution = match impact_bid {
        Some(bid) if bid > current_index => bid.checked_sub(current_index)?,
        _ => UsdPrice::ZERO,
    };

    let ask_contribution = match impact_ask {
        Some(ask) if current_index > ask => current_index.checked_sub(ask)?,
        _ => UsdPrice::ZERO,
    };

    let ipd = bid_contribution.checked_sub(ask_contribution)?;

    // S_new = S + α × IPD
    current_index.checked_add(alpha.checked_mul(ipd)?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_order_book::Dimensionless};

    // ---- exp_neg_approx tests ----

    #[test]
    fn exp_neg_zero() {
        let result = exp_neg_approx(Dimensionless::ZERO).unwrap();
        assert_eq!(result, Dimensionless::ONE);
    }

    #[test]
    fn exp_neg_max_tick() {
        // exp(-0.1) = 0.904837...
        let x = Dimensionless::new_permille(100); // 0.1
        let result = exp_neg_approx(x).unwrap();
        let expected = Dimensionless::new_raw(904_837); // 0.904837
        let diff = result.checked_sub(expected).unwrap().checked_abs().unwrap();
        assert!(diff <= Dimensionless::new_raw(1)); // within ±0.000001
    }

    #[test]
    fn exp_neg_mid() {
        // exp(-0.05) = 0.951229...
        let x = Dimensionless::new_permille(50); // 0.05
        let result = exp_neg_approx(x).unwrap();
        let expected = Dimensionless::new_raw(951_229); // 0.951229
        let diff = result.checked_sub(expected).unwrap().checked_abs().unwrap();
        assert!(diff <= Dimensionless::new_raw(1));
    }

    #[test]
    fn exp_neg_small() {
        // exp(-0.001) = 0.999001...
        let x = Dimensionless::new_permille(1); // 0.001
        let result = exp_neg_approx(x).unwrap();
        let expected = Dimensionless::new_raw(999_001); // 0.999001 (truncated)
        let diff = result.checked_sub(expected).unwrap().checked_abs().unwrap();
        assert!(diff <= Dimensionless::new_raw(1));
    }

    // ---- compute_ewma_index_price tests ----

    #[test]
    fn both_sides_index_below_bid() {
        // S=100, bid=102, ask=105, dt=3s → IPD=2, alpha≈0.00167 → small upward nudge
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(102)),
            Some(UsdPrice::new_int(105)),
            Duration::from_seconds(3),
        )
        .unwrap();
        assert!(result > UsdPrice::new_int(100));
        assert!(result < UsdPrice::new_percent(10_001)); // < 100.01
    }

    #[test]
    fn both_sides_index_above_ask() {
        // S=110, bid=102, ask=105, dt=3s → IPD=-5, pushes down
        let result = compute_ewma_index_price(
            UsdPrice::new_int(110),
            Some(UsdPrice::new_int(102)),
            Some(UsdPrice::new_int(105)),
            Duration::from_seconds(3),
        )
        .unwrap();
        assert!(result < UsdPrice::new_int(110));
        assert!(result > UsdPrice::new_percent(10_999)); // > 109.99
    }

    #[test]
    fn index_inside_spread() {
        // S=103, bid=102, ask=105 → IPD=0, no movement
        let result = compute_ewma_index_price(
            UsdPrice::new_int(103),
            Some(UsdPrice::new_int(102)),
            Some(UsdPrice::new_int(105)),
            Duration::from_seconds(3),
        )
        .unwrap();
        assert_eq!(result, UsdPrice::new_int(103));
    }

    #[test]
    fn bid_only_no_ask() {
        // S=100, bid=102, ask=None → IPD=2, pushes up
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(102)),
            None,
            Duration::from_seconds(3),
        )
        .unwrap();
        assert!(result > UsdPrice::new_int(100));
    }

    #[test]
    fn ask_only_no_bid() {
        // S=110, bid=None, ask=105 → IPD=-5, pushes down
        let result = compute_ewma_index_price(
            UsdPrice::new_int(110),
            None,
            Some(UsdPrice::new_int(105)),
            Duration::from_seconds(3),
        )
        .unwrap();
        assert!(result < UsdPrice::new_int(110));
    }

    #[test]
    fn neither_side() {
        // No signal → no movement
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            None,
            None,
            Duration::from_seconds(3),
        )
        .unwrap();
        assert_eq!(result, UsdPrice::new_int(100));
    }

    #[test]
    fn zero_delta_t() {
        // dt=0 → alpha=0 → no movement regardless of IPD
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(110)),
            Some(UsdPrice::new_int(115)),
            Duration::from_millis(0),
        )
        .unwrap();
        assert_eq!(result, UsdPrice::new_int(100));
    }

    #[test]
    fn delta_t_exceeds_cap() {
        // dt=7200s (2h), capped to 180s → x=0.1, alpha≈0.0952
        // IPD = max(110-100,0) - max(100-115,0) = 10
        // S_new ≈ 100 + 0.0952 * 10 = 100.952
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(110)),
            Some(UsdPrice::new_int(115)),
            Duration::from_seconds(7200),
        )
        .unwrap();
        assert!(result > UsdPrice::new_percent(10_090)); // > 100.90
        assert!(result < UsdPrice::new_int(101)); // < 101
    }

    #[test]
    fn max_single_tick_bound() {
        // Huge dt, huge IPD=100 → still capped at ~9.52% of IPD
        // S=100, bid=200, ask=200 → IPD=100, alpha≈0.0952
        // S_new ≈ 100 + 9.52 = 109.52
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(200)),
            Some(UsdPrice::new_int(200)),
            Duration::from_seconds(999_999),
        )
        .unwrap();
        assert!(result < UsdPrice::new_int(110));
    }
}
