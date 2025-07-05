use {
    crate::{
        FillingOutcome, INCOMING_ORDERS, LIMIT_ORDERS, MARKET_ORDERS, MAX_ORACLE_STALENESS,
        MatchingOutcome, MergedOrders, Order, OrderTrait, PAIRS, PassiveLiquidityPool, RESERVES,
        VOLUMES, VOLUMES_BY_USER, fill_orders, match_and_fill_market_orders, match_limit_orders,
    },
    dango_account_factory::AccountQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{Direction, LimitOrdersMatched, OrderFilled, PassiveOrdersFilled},
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Api, Coins, Denom, EventBuilder, Inner, IsZero, Message, Number, NumberConst,
        Order as IterationOrder, Response, StdError, StdResult, Storage, SudoCtx, TransferBuilder,
        Udec128, Uint128,
    },
    std::{
        collections::{BTreeSet, HashMap, hash_map::Entry},
        iter,
    },
};

const HALF: Udec128 = Udec128::new_percent(50);

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let app_cfg = ctx.querier.query_dango_config()?;

    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);
    let mut account_querier = AccountQuerier::new(app_cfg.addresses.account_factory, ctx.querier);

    let mut events = EventBuilder::new();
    let mut refunds = TransferBuilder::new();
    let mut volumes = HashMap::new();
    let mut volumes_by_username = HashMap::new();
    let mut fees = Coins::new();
    let mut fee_payments = TransferBuilder::new();

    // Collect incoming orders and clear the temporary storage.
    let incoming_orders = INCOMING_ORDERS.drain(ctx.storage, None, None)?;

    // Add incoming orders to the persistent storage.
    for (order_key, order) in incoming_orders.values() {
        debug_assert!(
            order.created_at_block_height == ctx.block.height,
            "incoming order was created in a previous block! creation height: {}, current height: {}",
            order.created_at_block_height,
            ctx.block.height
        );

        LIMIT_ORDERS.save(ctx.storage, order_key.clone(), order)?;
    }

    // Find all the unique pairs that have received new orders in the block.
    let pairs_with_limit_orders = incoming_orders
        .into_values()
        .map(|((pair, ..), _)| pair)
        .collect::<BTreeSet<_>>();

    // Find all the pairs that have market orders in the block.
    let pairs_with_market_orders = MARKET_ORDERS
        .keys(ctx.storage, None, None, IterationOrder::Ascending)
        .map(|res| res.map(|(pair, ..)| pair))
        .collect::<StdResult<BTreeSet<_>>>()?;

    // Loop through the pairs that have received new orders in the block.
    // Match and clear the orders for each of them.
    // TODO: spawn a thread for each pair to process them in parallel.
    for (base_denom, quote_denom) in pairs_with_limit_orders.union(&pairs_with_market_orders) {
        clear_orders_of_pair(
            ctx.storage,
            ctx.api,
            ctx.block.height,
            app_cfg.addresses.dex,
            &mut oracle_querier,
            &mut account_querier,
            app_cfg.maker_fee_rate.into_inner(),
            app_cfg.taker_fee_rate.into_inner(),
            base_denom.clone(),
            quote_denom.clone(),
            &mut events,
            &mut refunds,
            &mut fees,
            &mut fee_payments,
            &mut volumes,
            &mut volumes_by_username,
        )?;
    }

    // Save the updated volumes.
    for (address, volume) in volumes {
        VOLUMES.save(ctx.storage, (&address, ctx.block.timestamp), &volume)?;
        // TODO: purge volume data that are too old.
    }

    for (username, volume) in volumes_by_username {
        VOLUMES_BY_USER.save(ctx.storage, (&username, ctx.block.timestamp), &volume)?;
        // TODO: purge volume data that are too old.
    }

    Ok(Response::new()
        .may_add_message(if refunds.is_non_empty() {
            Some(refunds.into_message())
        } else {
            None
        })
        .may_add_message(if fee_payments.is_non_empty() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: fee_payments.into_batch(),
                },
                fees,
            )?)
        } else {
            None
        })
        .add_events(events)?)
}

fn clear_orders_of_pair(
    storage: &mut dyn Storage,
    api: &dyn Api,
    current_block_height: u64,
    dex_addr: Addr,
    oracle_querier: &mut OracleQuerier,
    account_querier: &mut AccountQuerier,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    base_denom: Denom,
    quote_denom: Denom,
    events: &mut EventBuilder,
    refunds: &mut TransferBuilder,
    fees: &mut Coins,
    fee_payments: &mut TransferBuilder,
    volumes: &mut HashMap<Addr, Uint128>,
    volumes_by_username: &mut HashMap<Username, Uint128>,
) -> anyhow::Result<()> {
    // --------------------------- 1. Prepare orders ---------------------------

    // Load the market orders for this pair.
    let mut market_bids = MARKET_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .drain(storage, None, None)?
        .into_iter()
        .peekable();
    let mut market_asks = MARKET_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .drain(storage, None, None)?
        .into_iter()
        .peekable();

    // Create iterators over user orders.
    //
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let bid_iter = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending)
        .map(|res| {
            let ((price, _), limit_order) = res?;
            Ok((price, limit_order))
        });
    let ask_iter = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((price, _), limit_order) = res?;
            Ok((price, limit_order))
        });

    // Create iterators over passive orders.
    //
    // If the pool doesn't have passive liquidity (reserve is `None`), or if
    // the order book reflection fails, simply use empty iterators. I.e. place
    // no passive liquidity orders.
    let reserve = RESERVES.may_load(storage, (&base_denom, &quote_denom))?;
    let (passive_bid_iter, passive_ask_iter) = match &reserve {
        Some(reserve) => {
            // Create the passive liquidity orders if the pair has a pool.
            let pair = PAIRS.load(storage, (&base_denom, &quote_denom))?;
            pair.reflect_curve(
                oracle_querier,
                base_denom.clone(),
                quote_denom.clone(),
                reserve,
            )
            .inspect_err(|err| {
                let msg = format!("ERROR: reflect curve failed! base denom: {base_denom}, quote denom: {quote_denom}, reserve: {reserve:?}, error: {err}");
                api.debug(dex_addr, &msg);
            })
            .unwrap_or_else(|_| (Box::new(iter::empty()) as _, Box::new(iter::empty()) as _))
        },
        None => (Box::new(iter::empty()) as _, Box::new(iter::empty()) as _),
    };

    // Merge the orders from users and from the passive pool.
    let mut merged_bid_iter =
        MergedOrders::new(bid_iter, passive_bid_iter, IterationOrder::Descending).peekable();
    let mut merged_ask_iter =
        MergedOrders::new(ask_iter, passive_ask_iter, IterationOrder::Ascending).peekable();

    // -------------------- 2. Match and fill market orders --------------------

    // Run the market order matching algorithm.
    // 1. Match market BUY orders against resting SELL limit orders.
    // 2. Match market SELL orders against resting BUY limit orders.
    let market_bid_filling_outcomes = match_and_fill_market_orders(
        &mut market_bids,
        &mut merged_ask_iter,
        Direction::Bid,
        maker_fee_rate,
        taker_fee_rate,
        current_block_height,
    )?;
    let market_ask_filling_outcomes = match_and_fill_market_orders(
        &mut market_asks,
        &mut merged_bid_iter,
        Direction::Ask,
        maker_fee_rate,
        taker_fee_rate,
        current_block_height,
    )?;

    // ------------------------- 3. Match limit orders -------------------------

    // Run the limit order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_limit_orders(merged_bid_iter, merged_ask_iter)?;

    // ------------------------- 4. Fill limit orders --------------------------

    // If matching orders were found, then we need to fill the orders. All orders
    // are filled at the clearing price.
    let limit_order_filling_outcomes = if let Some((lower_price, higher_price)) = range {
        // Choose the clearing price. Any price within `range` gives the same
        // volume (measured in the base asset). We can either take
        //
        // - the lower end,
        // - the higher end, or
        // - the midpoint of the range.
        //
        // Here we choose the midpoint.
        let clearing_price = lower_price.checked_add(higher_price)?.checked_mul(HALF)?;

        events.push(LimitOrdersMatched {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            clearing_price,
            volume,
        })?;

        fill_orders(
            bids,
            asks,
            clearing_price,
            volume,
            current_block_height,
            maker_fee_rate,
            taker_fee_rate,
        )?
    } else {
        vec![]
    };

    // ----------------------- 5. Update contract state ------------------------

    // Loop over all unmatched market orders and refund the users.
    for (_, market_order) in market_bids {
        refunds.insert(
            market_order.user,
            quote_denom.clone(),
            market_order.remaining,
        )?;
    }
    for (_, market_order) in market_asks {
        refunds.insert(
            market_order.user,
            base_denom.clone(),
            market_order.remaining,
        )?;
    }

    // Track the inflows and outflows of the dex.
    let mut inflows = Coins::new();
    let mut outflows = Coins::new();

    // Track the number of passive liquidity orders filled.
    let mut passive_bid_filling_outcomes_len = 0;
    let mut passive_ask_filling_outcomes_len = 0;

    // Handle order filling outcomes for the user placed orders.
    for FillingOutcome {
        order_direction,
        order,
        filled_base,
        filled_quote,
        refund_base,
        refund_quote,
        fee_base,
        fee_quote,
    } in market_bid_filling_outcomes
        .into_values()
        .chain(market_ask_filling_outcomes.into_values())
        .chain(limit_order_filling_outcomes)
    {
        if let Some((order_id, user)) = order.id_and_user() {
            fill_user_order(
                user,
                &base_denom,
                &quote_denom,
                refund_base,
                refund_quote,
                fee_base,
                fee_quote,
                refunds,
                fees,
                fee_payments,
            )?;

            let clearing_price = Udec128::checked_from_ratio(filled_quote, filled_base)?;
            let cleared = order.remaining().is_zero();

            // Emit event for filled user orders to be used by the frontend
            events.push(OrderFilled {
                user,
                id: order_id,
                kind: order.kind(),
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
                direction: order_direction,
                filled_base,
                filled_quote,
                refund_base,
                refund_quote,
                fee_base,
                fee_quote,
                clearing_price,
                cleared,
            })?;

            if let Order::Limit(limit_order) = order {
                if limit_order.remaining.is_zero() {
                    // Remove the order from the storage if it was fully filled
                    LIMIT_ORDERS.remove(
                        storage,
                        (
                            (base_denom.clone(), quote_denom.clone()),
                            order_direction,
                            limit_order.price,
                            order_id,
                        ),
                    )?;
                } else {
                    LIMIT_ORDERS.save(
                        storage,
                        (
                            (base_denom.clone(), quote_denom.clone()),
                            order_direction,
                            limit_order.price,
                            order_id,
                        ),
                        &limit_order,
                    )?;
                }
            }
        } else {
            match order_direction {
                Direction::Bid => passive_bid_filling_outcomes_len += 1,
                Direction::Ask => passive_ask_filling_outcomes_len += 1,
            }
            fill_passive_order(
                &base_denom,
                &quote_denom,
                order_direction,
                filled_base,
                filled_quote,
                &mut inflows,
                &mut outflows,
            )?;
        }

        update_trading_volumes(
            storage,
            api,
            dex_addr,
            oracle_querier,
            account_querier,
            &base_denom,
            filled_base,
            order.user().unwrap_or(dex_addr),
            volumes,
            volumes_by_username,
        )?;
    }

    if passive_bid_filling_outcomes_len > 0 || passive_ask_filling_outcomes_len > 0 {
        events.push(PassiveOrdersFilled {
            passive_bid_filling_outcomes_len,
            passive_ask_filling_outcomes_len,
        })?;
    }

    // Update the pool reserve.
    if inflows.is_non_empty() || outflows.is_non_empty() {
        RESERVES.update(storage, (&base_denom, &quote_denom), |mut reserve| {
            for inflow in inflows {
                reserve.checked_add(&inflow)?;
            }

            for outflow in outflows {
                reserve.checked_sub(&outflow)?;
            }

            Ok::<_, StdError>(reserve)
        })?;
    }

    Ok(())
}

/// Handle the `FillingOutcome` of a user order.
///
/// ## Returns
///
/// - `refund`: fund to be sent back to the user.
/// - `fee`: protocol fee to be transferred to the taxman contract.
fn fill_user_order(
    user: Addr,
    base_denom: &Denom,
    quote_denom: &Denom,
    refund_base: Uint128,
    refund_quote: Uint128,
    fee_base: Uint128,
    fee_quote: Uint128,
    refunds: &mut TransferBuilder,
    fees: &mut Coins,
    fee_payments: &mut TransferBuilder,
) -> StdResult<()> {
    // Handle fees.
    if fee_base.is_non_zero() {
        fees.insert((base_denom.clone(), fee_base))?;
        fee_payments.insert(user, base_denom.clone(), fee_base)?;
    }

    if fee_quote.is_non_zero() {
        fees.insert((quote_denom.clone(), fee_quote))?;
        fee_payments.insert(user, quote_denom.clone(), fee_quote)?;
    }

    // Handle refunds.
    refunds.insert(user, base_denom.clone(), refund_base)?;
    refunds.insert(user, quote_denom.clone(), refund_quote)
}

fn fill_passive_order(
    base_denom: &Denom,
    quote_denom: &Denom,
    order_direction: Direction,
    filled_base: Uint128,
    filled_quote: Uint128,
    inflows: &mut Coins,
    outflows: &mut Coins,
) -> StdResult<()> {
    // The order only exists in the storage if it's not owned by the dex, since
    // the passive orders are "virtual". If it is virtual, we need to update the
    // reserve.
    match order_direction {
        Direction::Bid => {
            inflows.insert((base_denom.clone(), filled_base))?;
            outflows.insert((quote_denom.clone(), filled_quote))?;
        },
        Direction::Ask => {
            inflows.insert((quote_denom.clone(), filled_quote))?;
            outflows.insert((base_denom.clone(), filled_base))?;
        },
    }

    Ok(())
}

/// Updates trading volumes for both user addresses and usernames
fn update_trading_volumes(
    storage: &mut dyn Storage,
    api: &dyn Api,
    dex_addr: Addr,
    oracle_querier: &mut OracleQuerier,
    account_querier: &mut AccountQuerier,
    base_denom: &Denom,
    filled: Uint128,
    order_user: Addr,
    volumes: &mut HashMap<Addr, Uint128>,
    volumes_by_username: &mut HashMap<Username, Uint128>,
) -> anyhow::Result<()> {
    // Query the base asset's oracle price.
    let base_asset_price = match oracle_querier.query_price(base_denom, None) {
        Err(err) => {
            let msg = format!("ERROR: failed to query price! denom: {base_denom}, error: {err}");
            api.debug(dex_addr, &msg);

            // If the query fails, simply do nothing and return, since we want to
            // ensure that `cron_execute` function doesn't fail.
            return Ok(());
        },
        Ok(price) => price,
    };

    // Calculate the volume in USD for the filled order.
    let new_volume = base_asset_price.value_of_unit_amount(filled)?.into_int();

    // Record trading volume for the user's address
    {
        match volumes.entry(order_user) {
            Entry::Occupied(mut v) => {
                v.get_mut().checked_add_assign(new_volume)?;
            },
            Entry::Vacant(v) => {
                let volume = VOLUMES
                    .prefix(&order_user)
                    .values(storage, None, None, IterationOrder::Descending)
                    .next()
                    .transpose()?
                    .unwrap_or(Uint128::ZERO)
                    .checked_add(new_volume)?;

                v.insert(volume);
            },
        }
    }

    // Record trading volume for the user's username, if the trader is a
    // single-signature account (skip for multisig accounts).
    if let Some(username) = account_querier
        .query_account(order_user)?
        .and_then(|account| account.params.owner())
    {
        match volumes_by_username.entry(username.clone()) {
            Entry::Occupied(mut v) => {
                v.get_mut().checked_add_assign(new_volume)?;
            },
            Entry::Vacant(v) => {
                let volume = VOLUMES_BY_USER
                    .prefix(username)
                    .values(storage, None, None, IterationOrder::Descending)
                    .next()
                    .transpose()?
                    .unwrap_or(Uint128::ZERO)
                    .checked_add(new_volume)?;

                v.insert(volume);
            },
        }
    }

    Ok(())
}
