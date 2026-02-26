use dango_types::{
    Dimensionless, UsdPrice, UsdValue,
    perps::Position,
};

/// Compute the ADL ranking score for a single position.
///
/// ```plain
/// score = (unrealized_pnl / equity) * (notional / equity)
/// ```
///
/// Higher score means the position is closed first during auto-deleveraging.
/// Returns zero for positions with non-positive PnL or non-positive equity.
pub fn compute_adl_score(
    position: &Position,
    oracle_price: UsdPrice,
    user_equity: UsdValue,
) -> grug::MathResult<Dimensionless> {
    let unrealized_pnl = super::compute_position_unrealized_pnl(position, oracle_price)?;

    if unrealized_pnl <= UsdValue::ZERO || user_equity <= UsdValue::ZERO {
        return Ok(Dimensionless::ZERO);
    }

    let notional = position.size.checked_abs()?.checked_mul(oracle_price)?;
    let pnl_pct: Dimensionless = unrealized_pnl.checked_div(user_equity)?;
    let effective_leverage: Dimensionless = notional.checked_div(user_equity)?;

    pnl_pct.checked_mul(effective_leverage)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{FundingPerUnit, Quantity},
    };

    fn make_position(size: i128, entry_price: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        }
    }

    #[test]
    fn long_in_profit() {
        // Long 1 BTC at $50k, oracle $55k → PnL = $5k
        let pos = make_position(1, 50_000);
        let score = compute_adl_score(&pos, UsdPrice::new_int(55_000), UsdValue::new_int(10_000))
            .unwrap();

        assert!(score > Dimensionless::ZERO);
    }

    #[test]
    fn short_in_profit() {
        // Short 1 BTC at $55k, oracle $50k → PnL = $5k
        let pos = make_position(-1, 55_000);
        let score = compute_adl_score(&pos, UsdPrice::new_int(50_000), UsdValue::new_int(10_000))
            .unwrap();

        assert!(score > Dimensionless::ZERO);
    }

    #[test]
    fn position_at_loss_returns_zero() {
        // Long 1 BTC at $50k, oracle $45k → PnL = -$5k
        let pos = make_position(1, 50_000);
        let score = compute_adl_score(&pos, UsdPrice::new_int(45_000), UsdValue::new_int(10_000))
            .unwrap();

        assert_eq!(score, Dimensionless::ZERO);
    }

    #[test]
    fn zero_equity_returns_zero() {
        let pos = make_position(1, 50_000);
        let score =
            compute_adl_score(&pos, UsdPrice::new_int(55_000), UsdValue::ZERO).unwrap();

        assert_eq!(score, Dimensionless::ZERO);
    }

    #[test]
    fn higher_profit_higher_score() {
        let pos_small = make_position(1, 50_000);
        let pos_big = make_position(1, 40_000);

        let equity = UsdValue::new_int(10_000);
        let oracle = UsdPrice::new_int(55_000);

        let score_small = compute_adl_score(&pos_small, oracle, equity).unwrap();
        // pos_big PnL = 55k - 40k = 15k vs pos_small PnL = 55k - 50k = 5k
        let score_big = compute_adl_score(&pos_big, oracle, equity).unwrap();

        assert!(score_big > score_small);
    }

    #[test]
    fn higher_leverage_higher_score() {
        // Same entry, same equity, but larger position → higher leverage → higher score
        let pos_small = make_position(1, 50_000);
        let pos_big = make_position(2, 50_000);

        let equity = UsdValue::new_int(10_000);
        let oracle = UsdPrice::new_int(55_000);

        let score_small = compute_adl_score(&pos_small, oracle, equity).unwrap();
        let score_big = compute_adl_score(&pos_big, oracle, equity).unwrap();

        assert!(score_big > score_small);
    }
}
