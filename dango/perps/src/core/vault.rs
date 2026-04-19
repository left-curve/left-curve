use {
    dango_types::{Dimensionless, Quantity, UsdPrice, UsdValue, perps::PairParam},
    grug::MathResult,
};

/// A single quote (one side of the vault's market-making order).
pub struct VaultQuote {
    pub price: UsdPrice,
    pub size: Quantity,
}

/// Compute the vault's bid and ask quotes for a single pair.
///
/// The vault never crosses the book: if the calculated bid >= `best_ask`,
/// it is clamped to `best_ask - tick_size`. Symmetrically for asks.
///
/// Uses inventory skew: when the vault has a directional position, order
/// sizes and spreads are tilted to encourage unwinding.
///
/// Mutates: nothing (pure function).
///
/// Returns: `(Option<bid>, Option<ask>)`. A side is `None` if the
/// pair's vault params are zero, or if clamping would produce a
/// non-positive price.
pub fn compute_vault_quotes(
    oracle_price: UsdPrice,
    pair_param: &PairParam,
    best_bid: Option<UsdPrice>,
    best_ask: Option<UsdPrice>,
    allocated_margin: UsdValue,
    position_size: Quantity,
) -> MathResult<(Option<VaultQuote>, Option<VaultQuote>)> {
    // Skip the pair entirely if any vault param is zero.
    if pair_param.vault_half_spread.is_zero()
        || pair_param.vault_max_quote_size.is_zero()
        || pair_param.vault_liquidity_weight.is_zero()
        || pair_param.tick_size.is_zero()
        || allocated_margin.is_zero()
    {
        return Ok((None, None));
    }

    // Compute inventory skew: clamp(position_size / max_skew_size, -1, 1).
    // When max_skew_size is zero, skew is zero (no inventory adjustment).
    let skew = if pair_param.vault_max_skew_size.is_zero() {
        Dimensionless::ZERO
    } else {
        position_size
            .checked_div(pair_param.vault_max_skew_size)?
            .clamp(Dimensionless::new_int(-1), Dimensionless::new_int(1))
    };

    // Per-side margin (split equally between bid and ask).
    let half_margin = allocated_margin.half();

    // Margin-constrained base size per side:
    // base_size = half_margin / (oracle_price * initial_margin_ratio)
    let margin_size = half_margin
        .checked_div(oracle_price)?
        .checked_div(pair_param.initial_margin_ratio)?;
    let base_size = margin_size.min(pair_param.vault_max_quote_size);

    if base_size.is_zero() {
        return Ok((None, None));
    }

    // Tilt sizes by skew: when long, reduce bid size, increase ask size.
    // bid_size = base_size * (1 - skew * size_skew_factor)
    // ask_size = base_size * (1 + skew * size_skew_factor)
    let skew_size_term = skew.checked_mul(pair_param.vault_size_skew_factor)?;

    let bid_size = base_size.checked_mul(Dimensionless::ONE.checked_sub(skew_size_term)?)?;
    let bid = compute_bid(oracle_price, pair_param, best_ask, bid_size, skew)?;

    let ask_size = base_size.checked_mul(Dimensionless::ONE.checked_add(skew_size_term)?)?;
    let ask = compute_ask(oracle_price, pair_param, best_bid, ask_size, skew)?;

    Ok((bid, ask))
}

/// Compute the vault's bid quote.
///
/// Mutates: nothing.
///
/// Returns: `Some(VaultQuote)` if a valid bid can be placed, `None` otherwise.
fn compute_bid(
    oracle_price: UsdPrice,
    pair_param: &PairParam,
    best_ask: Option<UsdPrice>,
    size: Quantity,
    skew: Dimensionless,
) -> MathResult<Option<VaultQuote>> {
    // Effective spread = vault_half_spread * (1 + skew * spread_skew_factor).
    // When long, bid spread widens (less likely to accumulate more).
    let effective_spread = pair_param.vault_half_spread.checked_mul(
        Dimensionless::ONE.checked_add(skew.checked_mul(pair_param.vault_spread_skew_factor)?)?,
    )?;

    // Raw bid = oracle_price * (1 - effective_spread).
    let raw_bid = oracle_price.checked_sub(oracle_price.checked_mul(effective_spread)?)?;

    // Snap down to nearest tick: floor(raw / tick) * tick.
    let remainder = raw_bid.checked_rem(pair_param.tick_size)?;
    let mut bid_price = raw_bid.checked_sub(remainder)?;

    // Clamp: bid must be strictly below best ask.
    if let Some(best_ask) = best_ask
        && bid_price >= best_ask
    {
        bid_price = best_ask.checked_sub(pair_param.tick_size)?;
    }

    // Skip if price is zero or negative.
    if bid_price.is_zero() || bid_price.is_negative() {
        return Ok(None);
    }

    // Skip if size is zero or negative (fully skewed away).
    if size.is_zero() || size.is_negative() {
        return Ok(None);
    }

    // Check minimum order size.
    let notional = size.checked_mul(bid_price)?;
    if notional < pair_param.min_order_size {
        return Ok(None);
    }

    // Bid size is positive (buy).
    Ok(Some(VaultQuote {
        price: bid_price,
        size,
    }))
}

/// Compute the vault's ask quote.
///
/// Mutates: nothing.
///
/// Returns: `Some(VaultQuote)` if a valid ask can be placed, `None` otherwise.
fn compute_ask(
    oracle_price: UsdPrice,
    pair_param: &PairParam,
    best_bid: Option<UsdPrice>,
    size: Quantity,
    skew: Dimensionless,
) -> MathResult<Option<VaultQuote>> {
    // Effective spread = vault_half_spread * (1 - skew * spread_skew_factor).
    // When long, ask spread tightens (more attractive to takers).
    let effective_spread = pair_param.vault_half_spread.checked_mul(
        Dimensionless::ONE.checked_sub(skew.checked_mul(pair_param.vault_spread_skew_factor)?)?,
    )?;

    // Raw ask = oracle_price * (1 + effective_spread).
    let raw_ask = oracle_price.checked_add(oracle_price.checked_mul(effective_spread)?)?;

    // Snap up to nearest tick: ceil(raw / tick) * tick.
    let remainder = raw_ask.checked_rem(pair_param.tick_size)?;
    let mut ask_price = if remainder.is_zero() {
        raw_ask
    } else {
        raw_ask
            .checked_sub(remainder)?
            .checked_add(pair_param.tick_size)?
    };

    // Clamp: ask must be strictly above best bid.
    if let Some(best_bid) = best_bid
        && ask_price <= best_bid
    {
        ask_price = best_bid.checked_add(pair_param.tick_size)?;
    }

    // Skip if size is zero or negative (fully skewed away).
    if size.is_zero() || size.is_negative() {
        return Ok(None);
    }

    // Check minimum order size.
    let notional = size.checked_mul(ask_price)?;
    if notional < pair_param.min_order_size {
        return Ok(None);
    }

    // Ask size is negative (sell).
    Ok(Some(VaultQuote {
        price: ask_price,
        size: size.checked_neg()?,
    }))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::Dimensionless};

    fn default_pair_param() -> PairParam {
        PairParam {
            tick_size: UsdPrice::new_int(1),
            vault_half_spread: Dimensionless::new_permille(10), // 1%
            vault_max_quote_size: Quantity::new_int(100),
            min_order_size: UsdValue::ZERO,
            initial_margin_ratio: Dimensionless::new_permille(100), // 10%
            vault_liquidity_weight: Dimensionless::new_int(1),
            ..Default::default()
        }
    }

    #[test]
    fn basic_quotes() {
        let pair_param = default_pair_param();
        let oracle_price = UsdPrice::new_int(1000);
        let allocated_margin = UsdValue::new_int(10_000);

        let (bid, ask) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            None,
            None,
            allocated_margin,
            Quantity::ZERO,
        )
        .unwrap();

        let bid = bid.unwrap();
        let ask = ask.unwrap();

        // Bid: 1000 * (1 - 0.01) = 990, snapped down to tick = 990.
        assert_eq!(bid.price, UsdPrice::new_int(990));
        assert!(bid.size.is_positive());

        // Ask: 1000 * (1 + 0.01) = 1010, snapped up to tick = 1010.
        assert_eq!(ask.price, UsdPrice::new_int(1010));
        assert!(ask.size.is_negative());
    }

    #[test]
    fn skip_when_spread_zero() {
        let pair_param = PairParam {
            vault_half_spread: Dimensionless::ZERO,
            ..default_pair_param()
        };

        let (bid, ask) = compute_vault_quotes(
            UsdPrice::new_int(1000),
            &pair_param,
            None,
            None,
            UsdValue::new_int(10_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert!(bid.is_none());
        assert!(ask.is_none());
    }

    #[test]
    fn skip_when_max_size_zero() {
        let pair_param = PairParam {
            vault_max_quote_size: Quantity::ZERO,
            ..default_pair_param()
        };

        let (bid, ask) = compute_vault_quotes(
            UsdPrice::new_int(1000),
            &pair_param,
            None,
            None,
            UsdValue::new_int(10_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert!(bid.is_none());
        assert!(ask.is_none());
    }

    #[test]
    fn bid_clamped_below_best_ask() {
        let pair_param = default_pair_param();
        let oracle_price = UsdPrice::new_int(1000);

        // Best ask is at 995, which is above the raw bid (990), so no clamping.
        let (bid, _) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            None,
            Some(UsdPrice::new_int(995)),
            UsdValue::new_int(10_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert_eq!(bid.unwrap().price, UsdPrice::new_int(990));

        // Best ask at 990 — would cross, so clamp to 990 - 1 = 989.
        let (bid, _) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            None,
            Some(UsdPrice::new_int(990)),
            UsdValue::new_int(10_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert_eq!(bid.unwrap().price, UsdPrice::new_int(989));
    }

    #[test]
    fn ask_clamped_above_best_bid() {
        let pair_param = default_pair_param();
        let oracle_price = UsdPrice::new_int(1000);

        // Best bid at 1015 — above the raw ask (1010), so clamp to 1015 + 1 = 1016.
        let (_, ask) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            Some(UsdPrice::new_int(1015)),
            None,
            UsdValue::new_int(10_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert_eq!(ask.unwrap().price, UsdPrice::new_int(1016));
    }

    #[test]
    fn size_capped_at_max_quote_size() {
        // Very large margin but small max_quote_size.
        let pair_param = PairParam {
            vault_max_quote_size: Quantity::new_int(5),
            ..default_pair_param()
        };

        let (bid, ask) = compute_vault_quotes(
            UsdPrice::new_int(1000),
            &pair_param,
            None,
            None,
            UsdValue::new_int(1_000_000),
            Quantity::ZERO,
        )
        .unwrap();

        assert_eq!(bid.unwrap().size, Quantity::new_int(5));
        assert_eq!(ask.unwrap().size, Quantity::new_int(-5));
    }

    #[test]
    fn size_constrained_by_margin() {
        // Margin constrains more than max_quote_size.
        // half_margin = 500, oracle = 1000, imr = 10%
        // margin_size = 500 / (1000 * 0.1) = 5
        let pair_param = PairParam {
            vault_max_quote_size: Quantity::new_int(100),
            ..default_pair_param()
        };

        let (bid, ask) = compute_vault_quotes(
            UsdPrice::new_int(1000),
            &pair_param,
            None,
            None,
            UsdValue::new_int(1000), // half_margin = 500
            Quantity::ZERO,
        )
        .unwrap();

        assert_eq!(bid.unwrap().size, Quantity::new_int(5));
        assert_eq!(ask.unwrap().size, Quantity::new_int(-5));
    }

    // ----------------------- inventory skew tests -----------------------

    /// Helper: pair param with skew enabled.
    /// base_size = min(5000 / (1000 * 0.1), 100) = 50 per side.
    fn skew_pair_param() -> PairParam {
        PairParam {
            vault_size_skew_factor: Dimensionless::new_permille(500), // 0.5
            vault_spread_skew_factor: Dimensionless::new_permille(300), // 0.3
            vault_max_skew_size: Quantity::new_int(50),
            ..default_pair_param()
        }
    }

    /// Zero position produces the same quotes regardless of skew parameters.
    ///
    /// Setup: oracle = $1000, margin = $10k, half_spread = 1%, base_size = 50.
    /// Skew params: size_factor = 0.5, spread_factor = 0.3, max_skew = 50.
    /// Position = 0 → skew = 0.
    ///
    /// |     | naive        | skew-based   |
    /// |-----|--------------|--------------|
    /// | bid | 50 @ $990    | 50 @ $990    |
    /// | ask | 50 @ $1010   | 50 @ $1010   |
    ///
    /// Effect: no adjustment because position is flat.
    #[test]
    fn zero_position_matches_naive() {
        let skew = skew_pair_param();
        let naive = default_pair_param();
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        let (sb, sa) =
            compute_vault_quotes(oracle, &skew, None, None, margin, Quantity::ZERO).unwrap();
        let (nb, na) =
            compute_vault_quotes(oracle, &naive, None, None, margin, Quantity::ZERO).unwrap();

        let (sb, sa) = (sb.unwrap(), sa.unwrap());
        let (nb, na) = (nb.unwrap(), na.unwrap());

        assert_eq!(sb.price, nb.price);
        assert_eq!(sa.price, na.price);
        assert_eq!(sb.size, nb.size);
        assert_eq!(sa.size, na.size);
    }

    /// Non-zero position with zero skew factors produces naive quotes.
    ///
    /// Setup: oracle = $1000, margin = $10k, base_size = 50.
    /// Skew params: all zero (disabled). Position = 30 (long).
    ///
    /// |     | naive        | skew-based   |
    /// |-----|--------------|--------------|
    /// | bid | 50 @ $990    | 50 @ $990    |
    /// | ask | 50 @ $1010   | 50 @ $1010   |
    ///
    /// Effect: position is ignored entirely when skew is disabled.
    #[test]
    fn zero_skew_factors_match_naive() {
        let pair_param = default_pair_param(); // skew factors default to zero
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        let (b_neutral, a_neutral) =
            compute_vault_quotes(oracle, &pair_param, None, None, margin, Quantity::ZERO).unwrap();
        let (b_long, a_long) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(30),
        )
        .unwrap();

        let (bn, an) = (b_neutral.unwrap(), a_neutral.unwrap());
        let (bl, al) = (b_long.unwrap(), a_long.unwrap());

        assert_eq!(bn.price, bl.price);
        assert_eq!(an.price, al.price);
        assert_eq!(bn.size, bl.size);
        assert_eq!(an.size, al.size);
    }

    /// Long position reduces bid size and increases ask size.
    ///
    /// Setup: oracle = $1000, margin = $10k, base_size = 50.
    /// Skew params: size_factor = 0.5, spread_factor = 0.3, max_skew = 50.
    /// Position = 25 (long) → skew = 0.5.
    ///
    /// |     | naive | skew-based |
    /// |-----|-------|------------|
    /// | bid | 50    | 37.5       |
    /// | ask | 50    | 62.5       |
    ///
    /// Effect: vault offers more on the sell side to unwind its long,
    /// while reducing the buy side that would deepen it.
    #[test]
    fn skew_tilts_sizes_when_long() {
        let pair_param = skew_pair_param();
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);
        let (bid, ask) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(25),
        )
        .unwrap();

        let bid = bid.unwrap();
        let ask = ask.unwrap();

        // bid_size < ask_size (in absolute value)
        assert!(bid.size.checked_abs().unwrap() < ask.size.checked_abs().unwrap());
    }

    /// Short position increases bid size and reduces ask size.
    ///
    /// Setup: oracle = $1000, margin = $10k, base_size = 50.
    /// Skew params: size_factor = 0.5, spread_factor = 0.3, max_skew = 50.
    /// Position = -25 (short) → skew = -0.5.
    ///
    /// |     | naive | skew-based |
    /// |-----|-------|------------|
    /// | bid | 50    | 62.5       |
    /// | ask | 50    | 37.5       |
    ///
    /// Effect: vault offers more on the buy side to unwind its short,
    /// while reducing the sell side that would deepen it.
    #[test]
    fn skew_tilts_sizes_when_short() {
        let pair_param = skew_pair_param();
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);
        let (bid, ask) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(-25),
        )
        .unwrap();

        let bid = bid.unwrap();
        let ask = ask.unwrap();

        // bid_size > ask_size (in absolute value)
        assert!(bid.size.checked_abs().unwrap() > ask.size.checked_abs().unwrap());
    }

    /// Long position widens bid spread and tightens ask spread.
    ///
    /// Setup: oracle = $1000, margin = $10k, half_spread = 1%.
    /// Skew params: size_factor = 0.5, spread_factor = 0.3, max_skew = 50.
    /// Position = 25 (long) → skew = 0.5.
    ///
    /// |     | naive       | skew-based  |
    /// |-----|-------------|-------------|
    /// | bid | 50 @ $990   | 37.5 @ $988 |
    /// | ask | 50 @ $1010  | 62.5 @ $1009|
    ///
    /// Effect: bid further from oracle (less likely to fill → less buying);
    /// ask closer to oracle (more likely to fill → more selling to unwind).
    #[test]
    fn skew_tilts_spreads_when_long() {
        let pair_param = skew_pair_param();
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        // Neutral quotes for reference.
        let (nb, na) =
            compute_vault_quotes(oracle, &pair_param, None, None, margin, Quantity::ZERO).unwrap();

        // Long position: bid should be further from oracle, ask closer.
        let (lb, la) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(25),
        )
        .unwrap();

        let (nb, na) = (nb.unwrap(), na.unwrap());
        let (lb, la) = (lb.unwrap(), la.unwrap());

        // Bid further from oracle (lower price).
        assert!(lb.price <= nb.price);
        // Ask closer to oracle (lower price than neutral ask).
        assert!(la.price <= na.price);
    }

    /// Skew clamps at ±1 when position exceeds max_skew_size.
    ///
    /// Setup: oracle = $1000, margin = $10k.
    /// Skew params: size_factor = 0.5, spread_factor = 0.3, max_skew = 50.
    ///
    /// |     | naive       | pos=50 (skew=1) | pos=100 (clamped to 1) |
    /// |-----|-------------|-----------------|------------------------|
    /// | bid | 50 @ $990   | 25 @ $987       | 25 @ $987              |
    /// | ask | 50 @ $1010  | 75 @ $1007      | 75 @ $1007             |
    ///
    /// Effect: beyond max_skew_size the vault doesn't skew any harder,
    /// preventing extreme behavior with very large positions.
    #[test]
    fn skew_saturates_at_max() {
        let pair_param = skew_pair_param();
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        let (b1, a1) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(50),
        )
        .unwrap();

        // Position = 2x max_skew_size (100) → skew still clamped to 1.
        let (b2, a2) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(100),
        )
        .unwrap();

        // Both should produce identical quotes.
        let (b1, a1) = (b1.unwrap(), a1.unwrap());
        let (b2, a2) = (b2.unwrap(), a2.unwrap());
        assert_eq!(b1.price, b2.price);
        assert_eq!(a1.price, a2.price);
        assert_eq!(b1.size, b2.size);
        assert_eq!(a1.size, a2.size);
    }

    /// At maximum skew with size_factor = 1.0, one side is fully disabled.
    ///
    /// Setup: oracle = $1000, margin = $10k, base_size = 50.
    /// Skew params: size_factor = 1.0, spread_factor = 0, max_skew = 50.
    /// Position = 50 (long) → skew = 1.0.
    ///
    /// |     | naive       | skew-based    |
    /// |-----|-------------|---------------|
    /// | bid | 50 @ $990   | None (size=0) |
    /// | ask | 50 @ $1010  | 100 @ $1010   |
    ///
    /// Effect: vault completely stops buying and doubles sell capacity.
    /// Most aggressive unwinding posture — all liquidity directed at
    /// reducing the long position.
    #[test]
    fn max_size_skew_disables_one_side() {
        let pair_param = PairParam {
            vault_size_skew_factor: Dimensionless::new_int(1),
            vault_spread_skew_factor: Dimensionless::ZERO,
            vault_max_skew_size: Quantity::new_int(50),
            ..default_pair_param()
        };
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        let (bid, ask) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(50), // skew = 1
        )
        .unwrap();

        // Bid side fully disabled (size = 0).
        assert!(bid.is_none());
        // Ask side still active.
        assert!(ask.is_some());
    }

    /// `vault_spread_skew_factor > 1` causes the tightened side to cross
    /// the oracle price at maximum skew, while `bid < ask` still holds.
    ///
    /// Setup: oracle = $1000, half_spread = 1%, spread_factor = 2.0,
    /// max_skew = 50, position = 50 (long) → skew = 1.
    ///
    /// Expected math:
    ///   bid = 1000 * (1 - 0.01 * (1 + 1*2)) = 1000 * 0.97 = 970
    ///   ask = 1000 * (1 + 0.01 * (1 - 1*2)) = 1000 * 0.99 = 990
    ///
    /// Note: ask < oracle (the vault is willing to sell below oracle to
    /// aggressively unwind its long), but bid < ask is preserved.
    #[test]
    fn spread_skew_factor_above_one_crosses_oracle_on_tightened_side() {
        let pair_param = PairParam {
            vault_spread_skew_factor: Dimensionless::new_int(2),
            vault_size_skew_factor: Dimensionless::ZERO,
            vault_max_skew_size: Quantity::new_int(50),
            ..default_pair_param()
        };
        let oracle = UsdPrice::new_int(1000);
        let margin = UsdValue::new_int(10_000);

        let (bid, ask) = compute_vault_quotes(
            oracle,
            &pair_param,
            None,
            None,
            margin,
            Quantity::new_int(50), // skew = 1 (max long)
        )
        .unwrap();

        let bid = bid.unwrap();
        let ask = ask.unwrap();

        assert_eq!(bid.price, UsdPrice::new_int(970));
        assert_eq!(ask.price, UsdPrice::new_int(990));

        assert!(ask.price < oracle, "ask should cross below oracle");
        assert!(bid.price > UsdPrice::ZERO, "bid must stay positive");
        assert!(bid.price < ask.price, "bid < ask invariant must hold");
    }
}
