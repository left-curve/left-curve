use {
    crate::{
        ASKS, BIDS, LAST_VAULT_ORDERS_UPDATE, NEXT_ORDER_ID, PAIR_IDS, PAIR_PARAMS, PARAM, STATE,
        USER_STATES,
        core::compute_vault_quotes,
        execute::{ORACLE, cancel_order::cancel_all_orders_for},
        price::may_invert_price,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        UsdValue,
        perps::{Order, OrderId},
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
pub fn on_oracle_update(ctx: MutableCtx) -> anyhow::Result<Response> {
    let last_update = LAST_VAULT_ORDERS_UPDATE.may_load(ctx.storage)?.unwrap_or(0);

    ensure!(
        ctx.block.height > last_update,
        "vault orders already updated this block"
    );

    let param = PARAM.load(ctx.storage)?;
    let state = STATE.load(ctx.storage)?;
    let pair_ids = PAIR_IDS.load(ctx.storage)?;

    let mut vault_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // Step 1: Cancel all existing vault orders.
    cancel_all_orders_for(ctx.storage, ctx.contract, &mut vault_state)?;

    // Step 2: Compute the vault's available margin.
    // After cancellation, reserved_margin is zero and all vault capital is in
    // state.vault_margin (already UsdValue).
    let vault_margin_value = state.vault_margin;

    // If vault_total_weight is zero, no pairs have weights configured — skip.
    if param.vault_total_weight.is_zero() || vault_margin_value.is_zero() {
        // Persist vault state (orders were cancelled).
        if vault_state.is_empty() {
            USER_STATES.remove(ctx.storage, ctx.contract);
        } else {
            USER_STATES.save(ctx.storage, ctx.contract, &vault_state)?;
        }

        LAST_VAULT_ORDERS_UPDATE.save(ctx.storage, &ctx.block.height)?;

        return Ok(Response::new());
    }

    // Step 3: Load the next order ID once before the loop.
    let mut next_order_id = NEXT_ORDER_ID.may_load(ctx.storage)?.unwrap_or(OrderId::ONE);

    // Step 4: Iterate each pair and place vault orders.
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

        // Compute vault quotes.
        let (bid, ask) =
            compute_vault_quotes(oracle_price, &pair_param, best_bid, best_ask, pair_margin)?;

        // Place bid order.
        if let Some(bid_quote) = bid {
            let stored_price = may_invert_price(bid_quote.price, true);
            let order = Order {
                user: ctx.contract,
                size: bid_quote.size,
                reduce_only: false,
                reserved_margin: UsdValue::ZERO,
            };

            BIDS.save(
                ctx.storage,
                (pair_id.clone(), stored_price, next_order_id),
                &order,
            )?;

            vault_state.open_order_count += 1;
            next_order_id.checked_add_assign(Uint64::ONE)?;
        }

        // Place ask order.
        if let Some(ask_quote) = ask {
            let order = Order {
                user: ctx.contract,
                size: ask_quote.size,
                reduce_only: false,
                reserved_margin: UsdValue::ZERO,
            };

            ASKS.save(
                ctx.storage,
                (pair_id.clone(), ask_quote.price, next_order_id),
                &order,
            )?;

            vault_state.open_order_count += 1;
            next_order_id.checked_add_assign(Uint64::ONE)?;
        }
    }

    // Step 5: Persist updated state.
    LAST_VAULT_ORDERS_UPDATE.save(ctx.storage, &ctx.block.height)?;
    NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

    if vault_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.contract);
    } else {
        USER_STATES.save(ctx.storage, ctx.contract, &vault_state)?;
    }

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::LAST_VAULT_ORDERS_UPDATE,
        grug::{Addr, Coins, MockContext},
    };

    const CONTRACT: Addr = Addr::mock(0);

    #[test]
    fn on_oracle_update_once_per_block() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default())
            .with_block_height(10);

        // Simulate a previous successful call at block 10.
        LAST_VAULT_ORDERS_UPDATE
            .save(&mut ctx.storage, &10)
            .unwrap();

        // Second call at the same block height should fail.
        let result = super::on_oracle_update(ctx.as_mutable());

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
