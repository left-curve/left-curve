use {
    dango_types::{Dimensionless, Quantity, perps::PairParam},
    grug::MathResult,
};

/// Compute the funding premium directly from the vault's inventory skew.
///
/// When the vault is the dominant maker, its skew-aware pricing determines
/// the mid-market price. Substituting the vault's bid/ask formulas into the
/// premium formula yields:
///
/// ```text
/// premium = -halfSpread × skew × spreadSkewFactor × fundingRateMultiplier
/// ```
///
/// where `skew = clamp(positionSize / maxSkewSize, -1, 1)`. The
/// `fundingRateMultiplier` is a governance-tunable scale that lets admins
/// dial funding up or down without altering the vault's quoting.
///
/// **Sign convention:** a positive vault position (long) produces a negative
/// premium, meaning shorts pay longs — crediting the vault for absorbed
/// inventory. Symmetric when short.
pub fn compute_vault_premium(
    vault_position_size: Quantity,
    pair_param: &PairParam,
) -> MathResult<Dimensionless> {
    let skew = if pair_param.vault_max_skew_size.is_zero() {
        Dimensionless::ZERO
    } else {
        vault_position_size
            .checked_div(pair_param.vault_max_skew_size)?
            .clamp(Dimensionless::new_int(-1), Dimensionless::new_int(1))
    };

    // premium = -(halfSpread * skew * spreadSkewFactor * fundingRateMultiplier)
    pair_param
        .vault_half_spread
        .checked_mul(skew)?
        .checked_mul(pair_param.vault_spread_skew_factor)?
        .checked_mul(pair_param.funding_rate_multiplier)?
        .checked_neg()
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{Dimensionless, Quantity, perps::PairParam},
    };

    /// Default pair param with vault skew enabled.
    ///
    /// half_spread = 1% (0.01), spread_skew_factor = 0.3, max_skew_size = 100,
    /// funding_rate_multiplier = 1 (identity).
    fn default_pair_param() -> PairParam {
        PairParam {
            vault_half_spread: Dimensionless::new_permille(10), // 1%
            vault_spread_skew_factor: Dimensionless::new_permille(300), // 0.3
            vault_max_skew_size: Quantity::new_int(100),
            funding_rate_multiplier: Dimensionless::ONE,
            ..Default::default()
        }
    }

    #[test]
    fn zero_position_gives_zero_premium() {
        let premium = compute_vault_premium(Quantity::ZERO, &default_pair_param()).unwrap();
        assert_eq!(premium, Dimensionless::ZERO);
    }

    #[test]
    fn long_position_gives_negative_premium() {
        // position = 50, skew = 0.5
        // premium = -(0.01 * 0.5 * 0.3) = -0.0015
        let premium = compute_vault_premium(Quantity::new_int(50), &default_pair_param()).unwrap();
        assert!(premium.is_negative());
        assert_eq!(premium, Dimensionless::new_raw(-1_500)); // -0.0015
    }

    #[test]
    fn short_position_gives_positive_premium() {
        // position = -50, skew = -0.5
        // premium = -(0.01 * (-0.5) * 0.3) = 0.0015
        let premium = compute_vault_premium(Quantity::new_int(-50), &default_pair_param()).unwrap();
        assert!(premium.is_positive());
        assert_eq!(premium, Dimensionless::new_raw(1_500)); // 0.0015
    }

    #[test]
    fn skew_saturates_at_positive_one() {
        let param = default_pair_param();
        // position = 100 → skew = 1.0
        let at_max = compute_vault_premium(Quantity::new_int(100), &param).unwrap();
        // position = 200 → skew clamped to 1.0
        let beyond_max = compute_vault_premium(Quantity::new_int(200), &param).unwrap();
        assert_eq!(at_max, beyond_max);
        // premium = -(0.01 * 1.0 * 0.3) = -0.003
        assert_eq!(at_max, Dimensionless::new_raw(-3_000));
    }

    #[test]
    fn skew_saturates_at_negative_one() {
        let param = default_pair_param();
        let at_min = compute_vault_premium(Quantity::new_int(-100), &param).unwrap();
        let beyond_min = compute_vault_premium(Quantity::new_int(-200), &param).unwrap();
        assert_eq!(at_min, beyond_min);
        // premium = -(0.01 * (-1.0) * 0.3) = 0.003
        assert_eq!(at_min, Dimensionless::new_raw(3_000));
    }

    #[test]
    fn zero_spread_skew_factor_gives_zero_premium() {
        let param = PairParam {
            vault_spread_skew_factor: Dimensionless::ZERO,
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        assert_eq!(premium, Dimensionless::ZERO);
    }

    #[test]
    fn zero_half_spread_gives_zero_premium() {
        let param = PairParam {
            vault_half_spread: Dimensionless::ZERO,
            // Need non-zero max_quote_size etc. so the vault would normally quote.
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        assert_eq!(premium, Dimensionless::ZERO);
    }

    #[test]
    fn zero_max_skew_size_gives_zero_premium() {
        let param = PairParam {
            vault_max_skew_size: Quantity::ZERO,
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        assert_eq!(premium, Dimensionless::ZERO);
    }

    #[test]
    fn multiplier_two_doubles_premium() {
        // Baseline premium with multiplier = 1 is -0.0015 (see
        // `long_position_gives_negative_premium`). Doubling the multiplier
        // should double the magnitude.
        let param = PairParam {
            funding_rate_multiplier: Dimensionless::new_int(2),
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        // premium = -(0.01 * 0.5 * 0.3 * 2) = -0.003
        assert_eq!(premium, Dimensionless::new_raw(-3_000));
    }

    #[test]
    fn multiplier_half_halves_premium() {
        let param = PairParam {
            funding_rate_multiplier: Dimensionless::new_permille(500), // 0.5
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        // premium = -(0.01 * 0.5 * 0.3 * 0.5) = -0.00075
        assert_eq!(premium, Dimensionless::new_raw(-750));
    }

    #[test]
    fn zero_funding_rate_multiplier_gives_zero_premium() {
        // With the multiplier at zero, the premium is zero regardless of skew
        // or spread. Lets governance disable funding without touching the
        // vault's quoting parameters.
        let param = PairParam {
            funding_rate_multiplier: Dimensionless::ZERO,
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        assert_eq!(premium, Dimensionless::ZERO);
    }

    #[test]
    fn large_multiplier_amplifies_premium() {
        // Multipliers above 1 amplify funding. Upstream clamping by
        // `max_abs_funding_rate` happens in `compute_funding_delta`, so
        // `compute_vault_premium` is free to return large magnitudes.
        let param = PairParam {
            funding_rate_multiplier: Dimensionless::new_int(100),
            ..default_pair_param()
        };
        let premium = compute_vault_premium(Quantity::new_int(50), &param).unwrap();
        // premium = -(0.01 * 0.5 * 0.3 * 100) = -0.15
        assert_eq!(premium, Dimensionless::new_raw(-150_000));
    }
}
