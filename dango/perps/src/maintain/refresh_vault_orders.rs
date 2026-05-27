use {
    crate::{
        core::{compute_available_margin, compute_vault_quotes},
        querier::NoCachePerpQuerier,
        state::{LAST_VAULT_ORDERS_UPDATE, PAIR_IDS, PAIR_PARAMS, PAIR_STATES, PARAM, USER_STATES},
        trade::{CancelAllOrdersOutcome, compute_cancel_all_orders_outcome},
    },
    anyhow::ensure,
    dango_order_book::{
        ASKS, BIDS, LimitOrder, NEXT_ORDER_ID, Quantity, ReasonForOrderRemoval, UsdValue,
        increase_liquidity_depths, may_invert_price,
    },
    grug_math::{Number as _, NumberConst, Uint64},
    grug_types::{MutableCtx, Order as IterationOrder, QuerierExt, Response},
};

pub fn refresh_vault_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    ensure!(
        ctx.sender == ctx.contract || ctx.sender == ctx.querier.query_owner()?,
        "only the perps contract itself or the chain owner may refresh vault orders"
    );

    ensure!(
        {
            let last_update = LAST_VAULT_ORDERS_UPDATE.may_load(ctx.storage)?.unwrap_or(0);
            ctx.block.height > last_update
        },
        "vault orders already updated this block"
    );

    let param = PARAM.load(ctx.storage)?;
    let pair_ids = PAIR_IDS.load(ctx.storage)?;

    let mut vault_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    // --------------- Step 1: Cancel all existing vault orders ----------------

    let CancelAllOrdersOutcome {
        user_state: updated_vault_state,
    } = compute_cancel_all_orders_outcome(
        ctx.storage,
        ctx.contract,
        &vault_state,
        None,
        ReasonForOrderRemoval::Canceled,
    )?;

    vault_state = updated_vault_state;

    // ------------- Step 2: Compute the vault's available margin --------------

    let vault_margin_value = {
        let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);
        compute_available_margin(&perp_querier, &vault_state)?
    };

    if param.vault_total_weight.is_zero() || !vault_margin_value.is_positive() {
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

        if pair_param.vault_liquidity_weight.is_zero() {
            continue;
        }

        let oracle_price = PAIR_STATES.load(ctx.storage, pair_id)?.index_price;

        let pair_margin = vault_margin_value
            .checked_mul(pair_param.vault_liquidity_weight)?
            .checked_div(param.vault_total_weight)?;

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

        let position_size = vault_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or(Quantity::ZERO);

        let (bid, ask) = compute_vault_quotes(
            oracle_price,
            &pair_param,
            best_bid,
            best_ask,
            pair_margin,
            position_size,
        )?;

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
        grug_types::{
            Addr, Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions,
            ResultExt,
        },
        std::collections::BTreeMap,
    };

    const CONTRACT: Addr = Addr::mock(0);
    const ORACLE: Addr = Addr::mock(1);
    const OWNER: Addr = Addr::mock(2);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(3),
            taxman: Addr::mock(4),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Nobody,
                instantiate: Permission::Nobody,
            },
            max_orphan_age: Duration::from_seconds(0),
        }
    }

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
            .with_config(mock_config())
    }

    #[test]
    fn rejects_unauthorized_sender() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default())
            .with_block_height(10);

        refresh_vault_orders(ctx.as_mutable()).should_fail_with_error(
            "only the perps contract itself or the chain owner may refresh vault orders",
        );
    }

    #[test]
    fn owner_can_trigger() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(OWNER)
            .with_funds(Coins::default())
            .with_block_height(10);

        PARAM.save(&mut ctx.storage, &Default::default()).unwrap();
        PAIR_IDS
            .save(&mut ctx.storage, &Default::default())
            .unwrap();

        refresh_vault_orders(ctx.as_mutable()).should_succeed();
    }

    #[test]
    fn contract_can_trigger() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(CONTRACT)
            .with_funds(Coins::default())
            .with_block_height(10);

        PARAM.save(&mut ctx.storage, &Default::default()).unwrap();
        PAIR_IDS
            .save(&mut ctx.storage, &Default::default())
            .unwrap();

        refresh_vault_orders(ctx.as_mutable()).should_succeed();
    }

    #[test]
    fn once_per_block() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(CONTRACT)
            .with_funds(Coins::default())
            .with_block_height(10);

        LAST_VAULT_ORDERS_UPDATE
            .save(&mut ctx.storage, &10)
            .unwrap();

        refresh_vault_orders(ctx.as_mutable())
            .should_fail_with_error("vault orders already updated this block");
    }
}
