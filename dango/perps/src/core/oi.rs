use {anyhow::ensure, dango_types::HumanAmount};

/// Check that the opening portion of an order does not violate the max OI
/// constraint. If violated, the order is rejected entirely.
///
/// - Positive `opening_size` increases long OI.
/// - Negative `opening_size` increases short OI.
/// - Zero `opening_size` always passes (nothing to open).
pub fn check_oi_constraint(
    opening_size: HumanAmount,
    long_oi: HumanAmount,
    short_oi: HumanAmount,
    max_abs_oi: HumanAmount,
) -> anyhow::Result<()> {
    if opening_size.is_positive() {
        ensure!(
            long_oi.checked_add(opening_size)? <= max_abs_oi,
            "max long OI exceeded"
        );
    } else if opening_size.is_negative() {
        ensure!(
            short_oi.checked_add(-opening_size)? <= max_abs_oi,
            "max short OI exceeded"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    // (opening, long_oi, short_oi, max_abs_oi, should_pass)
    #[test_case(  0,  50,  50, 100, true  ; "zero opening always passes")]
    #[test_case( 10,  80,   0, 100, true  ; "long within limit")]
    #[test_case( 10,  90,   0, 100, true  ; "long exactly at limit")]
    #[test_case( 10,  91,   0, 100, false ; "long exceeds limit")]
    #[test_case(-10,   0,  80, 100, true  ; "short within limit")]
    #[test_case(-10,   0,  90, 100, true  ; "short exactly at limit")]
    #[test_case(-10,   0,  91, 100, false ; "short exceeds limit")]
    fn check_oi_constraint_works(
        opening: i128,
        long_oi: i128,
        short_oi: i128,
        max_abs_oi: i128,
        should_pass: bool,
    ) {
        let result = check_oi_constraint(
            HumanAmount::new(opening),
            HumanAmount::new(long_oi),
            HumanAmount::new(short_oi),
            HumanAmount::new(max_abs_oi),
        );
        assert_eq!(result.is_ok(), should_pass);
    }
}
