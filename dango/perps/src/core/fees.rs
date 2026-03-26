use {
    dango_types::{Dimensionless, Quantity, UsdPrice, UsdValue},
    grug::MathResult,
    std::collections::BTreeMap,
};

/// Given a base fee rate, a tiered fee schedule, and the user's recent
/// volume, return the applicable fee rate. The highest qualifying tier
/// wins; if no tier is met, the base rate applies.
pub fn resolve_fee_rate(
    base_rate: Dimensionless,
    tiers: &BTreeMap<UsdValue, Dimensionless>,
    recent_volume: UsdValue,
) -> Dimensionless {
    tiers
        .iter()
        .rev()
        .find(|&(&threshold, _)| recent_volume >= threshold)
        .map(|(_, rate)| *rate)
        .unwrap_or(base_rate)
}

/// Compute the USD notional value of a fill.
pub fn compute_notional(fill_size: Quantity, exec_price: UsdPrice) -> MathResult<UsdValue> {
    fill_size.checked_abs()?.checked_mul(exec_price)
}

/// Given the fillable size of an order, the execution price, and the applicable
/// fee rate (maker or taker), compute the trading fee as a USD value.
pub fn compute_trading_fee(
    fill_size: Quantity,
    exec_price: UsdPrice,
    fee_rate: Dimensionless,
) -> MathResult<UsdValue> {
    compute_notional(fill_size, exec_price)?.checked_mul(fee_rate)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, std::collections::BTreeMap, test_case::test_case};

    #[test]
    fn resolve_fee_rate_empty_tiers() {
        let base = Dimensionless::new_permille(1);
        let tiers = BTreeMap::new();
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(1_000_000)),
            base
        );
    }

    #[test]
    fn resolve_fee_rate_below_all_thresholds() {
        let base = Dimensionless::new_permille(1);
        let tiers = BTreeMap::from([
            (UsdValue::new_int(100_000), Dimensionless::new_raw(800)),
            (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
        ]);
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(50_000)),
            base
        );
    }

    #[test]
    fn resolve_fee_rate_between_thresholds() {
        let base = Dimensionless::new_permille(1);
        let tier1_rate = Dimensionless::new_raw(800);
        let tiers = BTreeMap::from([
            (UsdValue::new_int(100_000), tier1_rate),
            (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
        ]);
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(500_000)),
            tier1_rate
        );
    }

    #[test]
    fn resolve_fee_rate_above_all_thresholds() {
        let base = Dimensionless::new_permille(1);
        let top_rate = Dimensionless::new_raw(500);
        let tiers = BTreeMap::from([
            (UsdValue::new_int(100_000), Dimensionless::new_raw(800)),
            (UsdValue::new_int(1_000_000), top_rate),
        ]);
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(5_000_000)),
            top_rate
        );
    }

    #[test]
    fn resolve_fee_rate_exactly_at_threshold() {
        let base = Dimensionless::new_permille(1);
        let tier1_rate = Dimensionless::new_raw(800);
        let tiers = BTreeMap::from([
            (UsdValue::new_int(100_000), tier1_rate),
            (UsdValue::new_int(1_000_000), Dimensionless::new_raw(500)),
        ]);
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(100_000)),
            tier1_rate
        );
    }

    #[test]
    fn resolve_fee_rate_negative_tier_rate() {
        // All-negative (rebate) tiers: higher volume → more generous rebate.
        let base = Dimensionless::new_raw(-100); // -1 bps
        let tier1_rate = Dimensionless::new_raw(-200); // -2 bps
        let tier2_rate = Dimensionless::new_raw(-500); // -5 bps
        let tiers = BTreeMap::from([
            (UsdValue::new_int(100_000), tier1_rate),
            (UsdValue::new_int(1_000_000), tier2_rate),
        ]);

        // Below all tiers → base rebate.
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(50_000)),
            base
        );

        // Between tiers → tier 1 rebate.
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(500_000)),
            tier1_rate
        );

        // Above all tiers → tier 2 (most generous) rebate.
        assert_eq!(
            resolve_fee_rate(base, &tiers, UsdValue::new_int(5_000_000)),
            tier2_rate
        );
    }

    // (fill_size, exec_price, fee_rate_raw, expected_raw)
    #[test_case(   0,      1,   1_000,          0 ; "zero fill")]
    #[test_case( 100,      1,   1_000,    100_000 ; "simple 0.1 percent fee")]
    #[test_case(-100,      1,   1_000,    100_000 ; "negative fill same result")]
    #[test_case(   1, 50_000,     500, 25_000_000 ; "high exec price")]
    #[test_case( 100,      1,  -1_000,   -100_000 ; "negative fee rate produces negative fee")]
    #[test_case(-100,      1,  -1_000,   -100_000 ; "negative fill and negative rate")]
    #[test_case(   1, 50_000,    -100, -5_000_000 ; "negative 1 bps on high price")]
    #[test_case( 100,      1,       0,          0 ; "zero fee rate")]
    fn compute_trading_fee_works(
        fill_size: i128,
        exec_price: i128,
        fee_rate_raw: i128,
        expected_raw: i128,
    ) {
        assert_eq!(
            compute_trading_fee(
                Quantity::new_int(fill_size),
                UsdPrice::new_int(exec_price),
                Dimensionless::new_raw(fee_rate_raw),
            )
            .unwrap(),
            UsdValue::new_raw(expected_raw),
        );
    }
}
