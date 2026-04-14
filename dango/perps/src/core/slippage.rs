use {anyhow::ensure, dango_types::Dimensionless};

/// Validate that `max_slippage` is in the range `[0, 1)`.
pub fn validate_slippage(max_slippage: Dimensionless) -> anyhow::Result<()> {
    ensure!(
        !max_slippage.is_negative(),
        "max slippage can't be negative: {max_slippage}"
    );

    ensure!(
        max_slippage < Dimensionless::ONE,
        "max slippage must be less than 1, got {max_slippage}"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug::ResultExt};

    #[test]
    fn accept_zero_slippage() {
        validate_slippage(Dimensionless::ZERO).should_succeed();
    }

    #[test]
    fn accept_50pct_slippage() {
        validate_slippage(Dimensionless::new_permille(500)).should_succeed();
    }

    #[test]
    fn reject_negative_slippage() {
        validate_slippage(Dimensionless::new_int(-1))
            .should_fail_with_error("max slippage can't be negative");
    }

    #[test]
    fn reject_100pct_slippage() {
        validate_slippage(Dimensionless::ONE)
            .should_fail_with_error("max slippage must be less than 1, got");
    }

    #[test]
    fn reject_150pct_slippage() {
        validate_slippage(Dimensionless::new_permille(1500))
            .should_fail_with_error("max slippage must be less than 1, got");
    }
}
