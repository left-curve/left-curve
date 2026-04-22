use {anyhow::ensure, dango_types::Dimensionless};

/// Validate that `max_slippage` is in the range `[0, pair_cap]`, where
/// `pair_cap` is the per-pair `max_market_slippage` parameter.
///
/// The absolute `[0, 1)` bound is retained as a sanity check so callers
/// that pass a nonsense value get a clear legacy-style error even if a
/// misconfigured `pair_cap` would otherwise admit it.
pub fn validate_slippage(
    max_slippage: Dimensionless,
    pair_cap: Dimensionless,
) -> anyhow::Result<()> {
    ensure!(
        !max_slippage.is_negative(),
        "max slippage can't be negative: {max_slippage}"
    );

    ensure!(
        max_slippage < Dimensionless::ONE,
        "max slippage must be less than 1, got {max_slippage}"
    );

    ensure!(
        max_slippage <= pair_cap,
        "max slippage {max_slippage} exceeds the pair cap {pair_cap}"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug::ResultExt};

    /// A generous default cap used across the legacy-style bound tests.
    /// The cap is `>= 1`... wait, cap must be `< 1` at configure time.
    /// Use 99.9% so every legacy value is below the cap and the legacy
    /// error path is the one that fires.
    fn permissive_cap() -> Dimensionless {
        Dimensionless::new_permille(999) // 99.9%
    }

    fn ten_pct() -> Dimensionless {
        Dimensionless::new_permille(100)
    }

    #[test]
    fn accept_zero_slippage() {
        validate_slippage(Dimensionless::ZERO, permissive_cap()).should_succeed();
    }

    #[test]
    fn accept_slippage_equal_to_cap() {
        validate_slippage(ten_pct(), ten_pct()).should_succeed();
    }

    #[test]
    fn accept_slippage_below_cap() {
        validate_slippage(Dimensionless::new_permille(50), ten_pct()).should_succeed();
    }

    #[test]
    fn reject_negative_slippage() {
        validate_slippage(Dimensionless::new_int(-1), permissive_cap())
            .should_fail_with_error("max slippage can't be negative");
    }

    #[test]
    fn reject_100pct_slippage() {
        validate_slippage(Dimensionless::ONE, permissive_cap())
            .should_fail_with_error("max slippage must be less than 1, got");
    }

    #[test]
    fn reject_150pct_slippage() {
        validate_slippage(Dimensionless::new_permille(1500), permissive_cap())
            .should_fail_with_error("max slippage must be less than 1, got");
    }

    #[test]
    fn reject_slippage_above_cap() {
        // 11% slippage against a 10% cap — rejected with the cap error.
        let err = validate_slippage(Dimensionless::new_permille(110), ten_pct())
            .unwrap_err()
            .to_string();
        assert!(err.contains("exceeds the pair cap"), "{err}");
        assert!(err.contains("0.11"), "{err}");
        assert!(err.contains("0.1"), "{err}");
    }

    #[test]
    fn reject_slippage_just_above_cap() {
        validate_slippage(
            Dimensionless::new_permille(100)
                .checked_add(Dimensionless::new_raw(1))
                .unwrap(),
            ten_pct(),
        )
        .should_fail_with_error("exceeds the pair cap");
    }

    /// Sanity: the legacy bound fires before the cap check when the cap
    /// itself is unexpectedly large (e.g., during a misconfiguration or
    /// an in-flight migration). This preserves the original error
    /// message for "nonsense" input.
    #[test]
    fn legacy_bound_fires_before_cap_check() {
        // Pass cap = 99.9% (max legal value) and slippage = 150%. The
        // `< 1` legacy check fires, not the cap check.
        let err = validate_slippage(Dimensionless::new_permille(1500), permissive_cap())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("max slippage must be less than 1"),
            "legacy bound should fire first: {err}"
        );
    }
}
