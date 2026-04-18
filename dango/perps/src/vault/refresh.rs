use {
    crate::{
        MAX_ORACLE_STALENESS,
        core::{compute_available_margin, compute_vault_quotes},
        liquidity_depth::increase_liquidity_depths,
        oracle,
        price::may_invert_price,
        querier::NoCachePerpQuerier,
        state::{
            ASKS, BIDS, LAST_VAULT_ORDERS_UPDATE, NEXT_ORDER_ID, PAIR_IDS, PAIR_PARAMS, PARAM,
            USER_STATES,
        },
        trade::{CancelAllOrdersOutcome, compute_cancel_all_orders_outcome},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, UsdValue,
        perps::{LimitOrder, ReasonForOrderRemoval},
    },
    grug::{MutableCtx, Number as _, NumberConst, Order as IterationOrder, Response, Uint64},
};

/// Entry point for vault market-making, triggered at the beginning of each
/// block after the oracle update.
///
/// 1. Cancels all existing vault orders.
/// 2. Computes available margin for the vault.
/// 3. For each trading pair, places fresh bid/ask limit orders based on the
///    oracle price and the pair's market-making parameters.
///
/// Mutates: `USER_STATES[contract]`, `BIDS`, `ASKS`, `NEXT_ORDER_ID`.
///
/// Returns: empty `Response` (no token transfers).
pub fn refresh_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    let oracle_addr = oracle(ctx.querier);

    // Only the oracle contract (after feeding fresh prices) may trigger a vault
    // refresh. Without this check any account could consume the once-per-block
    // refresh token on stale prices, causing the oracle's legitimate refresh
    // submessage to be rejected.
    ensure!(
        ctx.sender == oracle_addr,
        "only the oracle contract may refresh vault orders"
    );

    let last_update = LAST_VAULT_ORDERS_UPDATE.may_load(ctx.storage)?.unwrap_or(0);

    ensure!(
        ctx.block.height > last_update,
        "vault orders already updated this block"
    );

    let param = PARAM.load(ctx.storage)?;
    let pair_ids = PAIR_IDS.load(ctx.storage)?;

    let mut vault_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(oracle_addr, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // --------------- Step 1: Cancel all existing vault orders ----------------

    let CancelAllOrdersOutcome {
        user_state: updated_vault_state,
    } = compute_cancel_all_orders_outcome(
        ctx.storage,
        ctx.contract,
        &vault_state,
        None, // Vault order churn is not user-facing — no events emitted.
        ReasonForOrderRemoval::Canceled,
    )?;

    vault_state = updated_vault_state;

    // ------------- Step 2: Compute the vault's available margin --------------

    // Compute available margin: equity minus margin consumed by existing
    // positions. After cancellation reserved_margin is zero, so the formula
    // simplifies to: max(0, equity - used_margin).
    let vault_margin_value = {
        let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);
        compute_available_margin(&mut oracle_querier, &perp_querier, &vault_state)?
    };

    // If vault_total_weight is zero, no pairs have weights configured — skip.
    if param.vault_total_weight.is_zero() || !vault_margin_value.is_positive() {
        // Persist vault state (orders were cancelled).
        if vault_state.is_empty() {
            USER_STATES.remove(ctx.storage, ctx.contract)?;
        } else {
            USER_STATES.save(ctx.storage, ctx.contract, &vault_state)?;
        }

        LAST_VAULT_ORDERS_UPDATE.save(ctx.storage, &ctx.block.height)?;

        return Ok(Response::new());
    }

    // ----------- Step 3: Iterate each pair and place vault orders ------------

    let mut next_order_id = NEXT_ORDER_ID.load(ctx.storage)?;

    for pair_id in &pair_ids {
        let pair_param = PAIR_PARAMS.load(ctx.storage, pair_id)?;

        // Skip pairs with zero weight.
        if pair_param.vault_liquidity_weight.is_zero() {
            continue;
        }

        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

        // Compute this pair's allocated margin.
        let pair_margin = vault_margin_value
            .checked_mul(pair_param.vault_liquidity_weight)?
            .checked_div(param.vault_total_weight)?;

        // Read best bid (un-inverted) and best ask from the book.
        let best_bid = BIDS
            .prefix(pair_id.clone())
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .next()
            .transpose()?
            .map(|((stored_price, _), _)| may_invert_price(stored_price, true));

        let best_ask = ASKS
            .prefix(pair_id.clone())
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .next()
            .transpose()?
            .map(|((stored_price, _), _)| stored_price);

        // Vault's current position for this pair (zero if none).
        let position_size = vault_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or(Quantity::ZERO);

        // Compute vault quotes with inventory skew.
        let (bid, ask) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            best_bid,
            best_ask,
            pair_margin,
            position_size,
        )?;

        // Place bid order.
        if let Some(bid_quote) = bid {
            let stored_price = may_invert_price(bid_quote.price, true);
            let order = LimitOrder {
                user: ctx.contract,
                size: bid_quote.size,
                reduce_only: false,
                reserved_margin: UsdValue::ZERO,
                created_at: ctx.block.timestamp,
                tp: None,
                sl: None,
                client_order_id: None,
            };

            BIDS.save(
                ctx.storage,
                (pair_id.clone(), stored_price, next_order_id),
                &order,
            )?;

            increase_liquidity_depths(
                ctx.storage,
                pair_id,
                true,
                bid_quote.price,
                bid_quote.size.checked_abs()?,
                &pair_param.bucket_sizes,
            )?;

            vault_state.open_order_count += 1;
            next_order_id.checked_add_assign(Uint64::ONE)?;
        }

        // Place ask order.
        if let Some(ask_quote) = ask {
            let order = LimitOrder {
                user: ctx.contract,
                size: ask_quote.size,
                reduce_only: false,
                reserved_margin: UsdValue::ZERO,
                created_at: ctx.block.timestamp,
                tp: None,
                sl: None,
                client_order_id: None,
            };

            ASKS.save(
                ctx.storage,
                (pair_id.clone(), ask_quote.price, next_order_id),
                &order,
            )?;

            increase_liquidity_depths(
                ctx.storage,
                pair_id,
                false,
                ask_quote.price,
                ask_quote.size.checked_abs()?,
                &pair_param.bucket_sizes,
            )?;

            vault_state.open_order_count += 1;
            next_order_id.checked_add_assign(Uint64::ONE)?;
        }
    }

    // --------------------- Step 4: Persist updated state ---------------------

    LAST_VAULT_ORDERS_UPDATE.save(ctx.storage, &ctx.block.height)?;

    NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

    if vault_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.contract)?;
    } else {
        USER_STATES.save(ctx.storage, ctx.contract, &vault_state)?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            num_pairs = pair_ids.len(),
            open_orders = vault_state.open_order_count,
            "Vault orders refreshed"
        );
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_DURATION_VAULT_REFRESH)
            .record(start.elapsed().as_secs_f64());
    }

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::config::AppConfig,
        grug::{Addr, Coins, MockContext, MockQuerier, ResultExt},
    };

    const CONTRACT: Addr = Addr::mock(0);
    const ORACLE: Addr = Addr::mock(1);

    /// Build a mock querier whose `AppConfig` returns `ORACLE` as the oracle address.
    fn mock_querier() -> MockQuerier {
        MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: dango_types::config::AppAddresses {
                    oracle: ORACLE,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
    }

    #[test]
    fn refresh_orders_rejects_unauthorized_sender() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default())
            .with_block_height(10);

        refresh_orders(ctx.as_mutable())
            .should_fail_with_error("only the oracle contract may refresh vault orders");
    }

    #[test]
    fn refresh_orders_once_per_block() {
        // Use the contract itself as sender (self-call path).
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(ORACLE) // note: refreshing order is permissioned to only the oracle contract.
            .with_funds(Coins::default())
            .with_block_height(10);

        // Simulate a previous successful call at block 10.
        LAST_VAULT_ORDERS_UPDATE
            .save(&mut ctx.storage, &10)
            .unwrap();

        // Second call at the same block height should fail.
        let result = refresh_orders(ctx.as_mutable());

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already updated this block"),
            "expected 'already updated this block' error"
        );
    }
}
