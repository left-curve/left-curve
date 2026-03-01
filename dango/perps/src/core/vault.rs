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

    // Per-side margin (split equally between bid and ask).
    let half_margin = allocated_margin.checked_div(Dimensionless::new_int(2))?;

    // Margin-constrained size per side:
    // size = half_margin / (oracle_price * initial_margin_ratio)
    let margin_size = half_margin
        .checked_div(oracle_price)?
        .checked_div(pair_param.initial_margin_ratio)?;
    let capped_size = margin_size.min(pair_param.vault_max_quote_size);

    if capped_size.is_zero() {
        return Ok((None, None));
    }

    let bid = compute_bid(oracle_price, pair_param, best_ask, capped_size)?;

    let ask = compute_ask(oracle_price, pair_param, best_bid, capped_size)?;

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
) -> MathResult<Option<VaultQuote>> {
    // Raw bid = oracle_price * (1 - vault_half_spread).
    let raw_bid =
        oracle_price.checked_sub(oracle_price.checked_mul(pair_param.vault_half_spread)?)?;

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
) -> MathResult<Option<VaultQuote>> {
    // Raw ask = oracle_price * (1 + vault_half_spread).
    let raw_ask =
        oracle_price.checked_add(oracle_price.checked_mul(pair_param.vault_half_spread)?)?;

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

        let (bid, ask) =
            compute_vault_quotes(oracle_price, &pair_param, None, None, allocated_margin).unwrap();

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
        )
        .unwrap();

        assert_eq!(bid.unwrap().size, Quantity::new_int(5));
        assert_eq!(ask.unwrap().size, Quantity::new_int(-5));
    }
}
