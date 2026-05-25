use {
    dango_order_book::{Dimensionless, UsdPrice},
    grug_math::MathResult,
    grug_types::Duration,
};

/// EWMA time constant (tau): 30 minutes. Controls how quickly the index price
/// converges toward the order book. After tau of sustained pressure, the index
/// closes ~63% of the gap.
pub const INDEX_TIME_CONSTANT: Duration = Duration::from_minutes(30);

/// Denominator of the maximum fraction of the time constant that a single tick
/// can cover. Caps the per-tick EWMA weight to
/// `1 - exp(-1/MAX_TICK_FRACTION_DENOMINATOR)` ~= 9.52%, preventing large jumps
/// after long pauses.
pub const MAX_TICK_FRACTION_DENOMINATOR: u128 = 10; // c = 1/10 = 0.1

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

    // Δt* = min(Δt, c × τ)
    let c_tau_millis = tau_millis / MAX_TICK_FRACTION_DENOMINATOR;
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

    /// When delta_t equals the cap exactly (c * tau = 0.1 * 30min = 180s),
    /// the EWMA weight is 1 - exp(-0.1) ~= 9.52%. The result should match
    /// the theoretical value.
    #[test]
    fn c1_delta_t_exactly_at_cap() {
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(110)),
            Some(UsdPrice::new_int(115)),
            Duration::from_seconds(180),
        )
        .unwrap();
        assert!(result > UsdPrice::new_percent(10_090)); // > 100.90
        assert!(result < UsdPrice::new_int(101)); // < 101
    }

    /// At 181s the time delta is capped to 180s, so the result must equal
    /// the 180s result exactly. At 179s (below the cap) the result must be
    /// strictly smaller since a shorter interval yields a smaller EWMA weight.
    #[test]
    fn c2_cap_boundary_179s_vs_181s() {
        let s = UsdPrice::new_int(100);
        let bid = Some(UsdPrice::new_int(110));
        let ask = Some(UsdPrice::new_int(115));

        let at_179 = compute_ewma_index_price(s, bid, ask, Duration::from_seconds(179)).unwrap();
        let at_180 = compute_ewma_index_price(s, bid, ask, Duration::from_seconds(180)).unwrap();
        let at_181 = compute_ewma_index_price(s, bid, ask, Duration::from_seconds(181)).unwrap();

        // 181s is capped to 180s, so result must equal 180s exactly.
        assert_eq!(at_181, at_180);
        // 179s is below the cap, so it yields a slightly smaller alpha.
        assert!(at_179 < at_180);
    }

    /// Per the trade.xyz spec, after one time constant (tau = 30 min) of
    /// sustained one-sided pressure, the index converges ~63% of the gap
    /// toward the order book. With S_0 = 100 and a target of 200, after
    /// 600 ticks of 3s each the result should be approximately 163.
    #[test]
    fn c3_sustained_convergence_over_tau() {
        let mut s = UsdPrice::new_int(100);
        let bid = Some(UsdPrice::new_int(200));
        let ask = Some(UsdPrice::new_int(200));
        let dt = Duration::from_seconds(3);

        for _ in 0..600 {
            s = compute_ewma_index_price(s, bid, ask, dt).unwrap();
        }

        let target = UsdPrice::new_int(163);
        let tolerance = UsdPrice::new_int(2);
        assert!(
            s > target.checked_sub(tolerance).unwrap()
                && s < target.checked_add(tolerance).unwrap(),
            "after tau of pressure, expected ~163, got {s}"
        );
    }

    /// When the index is below the bid but well inside the ask, only the bid
    /// side contributes to IPD. The ask term is zero because the index is not
    /// above the ask.
    #[test]
    fn c4_asymmetric_below_bid_inside_ask() {
        let result = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(105)),
            Some(UsdPrice::new_int(120)),
            Duration::from_seconds(3),
        )
        .unwrap();
        assert!(result > UsdPrice::new_int(100));
        assert!(result < UsdPrice::new_int(101));
    }

    /// Equal displacement on opposite sides of the spread must produce equal
    /// magnitude of movement. Setup A has IPD = +10 (below bid); setup B has
    /// IPD = -10 (above ask). The absolute price change must be the same.
    #[test]
    fn c5_symmetry_equal_displacement() {
        let dt = Duration::from_seconds(3);

        let result_a = compute_ewma_index_price(
            UsdPrice::new_int(100),
            Some(UsdPrice::new_int(110)),
            Some(UsdPrice::new_int(115)),
            dt,
        )
        .unwrap();

        let result_b = compute_ewma_index_price(
            UsdPrice::new_int(125),
            Some(UsdPrice::new_int(110)),
            Some(UsdPrice::new_int(115)),
            dt,
        )
        .unwrap();

        let delta_a = result_a.checked_sub(UsdPrice::new_int(100)).unwrap();
        let delta_b = UsdPrice::new_int(125).checked_sub(result_b).unwrap();
        assert_eq!(delta_a, delta_b);
    }
}
