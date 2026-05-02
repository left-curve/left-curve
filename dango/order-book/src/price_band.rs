use {
    crate::{Dimensionless, UsdPrice},
    anyhow::ensure,
};

/// Validate that `limit_price` is within the allowed deviation of `oracle_price`.
///
/// The invariant enforced is:
///
/// ```plain
/// |limit_price - oracle_price| <= oracle_price * max_deviation
/// ```
///
/// Equivalently, `limit_price` must fall inside
/// `[oracle_price * (1 - max_deviation), oracle_price * (1 + max_deviation)]`.
///
/// This check subsumes a positivity check on `limit_price`: any
/// `max_deviation < 1` (enforced at configure time) implies the lower bound
/// is strictly positive, so a zero or negative `limit_price` is rejected.
///
/// The caller is responsible for passing a `max_deviation` already in the
/// validated range `(0, 1)` — see `validate_pair_param` in the `maintain`
/// module. A zero or negative `max_deviation` would reject every price; a
/// `max_deviation >= 1` would produce a non-positive lower bound and admit
/// pathological prices.
pub fn check_price_band(
    limit_price: UsdPrice,
    oracle_price: UsdPrice,
    max_deviation: Dimensionless,
) -> anyhow::Result<()> {
    let upper_factor = Dimensionless::ONE.checked_add(max_deviation)?;
    let lower_factor = Dimensionless::ONE.checked_sub(max_deviation)?;

    let upper_bound = oracle_price.checked_mul(upper_factor)?;
    let lower_bound = oracle_price.checked_mul(lower_factor)?;

    ensure!(
        limit_price >= lower_bound && limit_price <= upper_bound,
        "limit price {limit_price} deviates too far from oracle price {oracle_price}: allowed range [{lower_bound}, {upper_bound}], max deviation {max_deviation}",
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug::ResultExt};

    /// oracle = 100, max_deviation = 10%
    /// Allowed range: [90, 110]
    fn oracle() -> UsdPrice {
        UsdPrice::new_int(100)
    }

    fn band_10pct() -> Dimensionless {
        Dimensionless::new_permille(100)
    }

    #[test]
    fn accept_limit_equal_to_oracle() {
        check_price_band(UsdPrice::new_int(100), oracle(), band_10pct()).should_succeed();
    }

    #[test]
    fn accept_limit_at_upper_bound() {
        check_price_band(UsdPrice::new_int(110), oracle(), band_10pct()).should_succeed();
    }

    #[test]
    fn accept_limit_at_lower_bound() {
        check_price_band(UsdPrice::new_int(90), oracle(), band_10pct()).should_succeed();
    }

    #[test]
    fn accept_limit_just_inside_upper_bound() {
        check_price_band(UsdPrice::new_int(109), oracle(), band_10pct()).should_succeed();
    }

    #[test]
    fn accept_limit_just_inside_lower_bound() {
        check_price_band(UsdPrice::new_int(91), oracle(), band_10pct()).should_succeed();
    }

    #[test]
    fn reject_limit_just_above_upper_bound() {
        check_price_band(UsdPrice::new_raw(110_000_001), oracle(), band_10pct())
            .should_fail_with_error("deviates too far");
    }

    #[test]
    fn reject_limit_just_below_lower_bound() {
        check_price_band(UsdPrice::new_raw(89_999_999), oracle(), band_10pct())
            .should_fail_with_error("deviates too far");
    }

    #[test]
    fn reject_limit_at_double_oracle() {
        check_price_band(UsdPrice::new_int(200), oracle(), band_10pct())
            .should_fail_with_error("deviates too far");
    }

    #[test]
    fn reject_limit_at_one_tenth_oracle() {
        check_price_band(UsdPrice::new_int(10), oracle(), band_10pct())
            .should_fail_with_error("deviates too far");
    }

    /// Zero limit price is outside any `max_deviation < 1`, so the band
    /// check subsumes a positivity check.
    #[test]
    fn reject_zero_limit_price() {
        check_price_band(UsdPrice::ZERO, oracle(), band_10pct())
            .should_fail_with_error("deviates too far");
    }

    /// Even the widest legal band — just below 100% — rejects zero, because
    /// `lower_bound = oracle * (1 - 99.9%) > 0`.
    #[test]
    fn zero_rejected_even_at_widest_legal_band() {
        check_price_band(
            UsdPrice::ZERO,
            oracle(),
            Dimensionless::new_permille(999), // 99.9%
        )
        .should_fail_with_error("deviates too far");
    }

    /// A 50% band around oracle = 100 admits [50, 150].
    #[test]
    fn wide_band_50pct_upper_ok() {
        check_price_band(
            UsdPrice::new_int(150),
            oracle(),
            Dimensionless::new_permille(500),
        )
        .should_succeed();
    }

    #[test]
    fn wide_band_50pct_lower_ok() {
        check_price_band(
            UsdPrice::new_int(50),
            oracle(),
            Dimensionless::new_permille(500),
        )
        .should_succeed();
    }

    #[test]
    fn wide_band_50pct_rejects_at_151() {
        check_price_band(
            UsdPrice::new_raw(150_000_001),
            oracle(),
            Dimensionless::new_permille(500),
        )
        .should_fail_with_error("deviates too far");
    }

    /// Narrow band 0.1% around oracle = 100 admits [99.9, 100.1].
    /// A limit at 101 (1% away) is well outside.
    #[test]
    fn narrow_band_rejects_1pct_away() {
        check_price_band(
            UsdPrice::new_int(101),
            oracle(),
            Dimensionless::new_permille(1), // 0.1%
        )
        .should_fail_with_error("deviates too far");
    }

    /// With a large oracle price, the absolute range scales proportionally.
    #[test]
    fn large_oracle_scaled_range() {
        let oracle = UsdPrice::new_int(50_000);
        let band = Dimensionless::new_permille(100); // 10%
        check_price_band(UsdPrice::new_int(45_000), oracle, band).should_succeed();
        check_price_band(UsdPrice::new_int(55_000), oracle, band).should_succeed();
        check_price_band(UsdPrice::new_raw(44_999_999_999), oracle, band)
            .should_fail_with_error("deviates too far");
        check_price_band(UsdPrice::new_raw(55_000_000_001), oracle, band)
            .should_fail_with_error("deviates too far");
    }
}
