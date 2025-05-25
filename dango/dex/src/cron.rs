use {
    crate::{
        FillingOutcome, INCOMING_ORDERS, LIMIT_ORDERS, MatchingOutcome, MergedOrders, PAIRS,
        PassiveLiquidityPool, RESERVES, VOLUMES, VOLUMES_BY_USER, fill_orders, match_orders,
    },
    dango_account_factory::AccountQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{Direction, OrderFilled, OrdersMatched},
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coin, Coins, Denom, EventBuilder, Inner, IsZero, Message, MultiplyFraction, Number,
        NumberConst, Order as IterationOrder, Response, StdError, Storage, SudoCtx, Udec128,
        Uint128,
    },
    std::{
        collections::{BTreeMap, BTreeSet, HashMap, hash_map::Entry},
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
    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier);
    let mut account_querier = AccountQuerier::new(app_cfg.addresses.account_factory, ctx.querier);

    let mut events = EventBuilder::new();
    let mut refunds = BTreeMap::new();
    let mut volumes = HashMap::new();
    let mut volumes_by_username = HashMap::new();
    let mut fees = Coins::new();
    let mut fee_payments = BTreeMap::new();

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
    let pairs = incoming_orders
        .into_values()
        .map(|((pair, ..), _)| pair)
        .collect::<BTreeSet<_>>();

    // Loop through the pairs that have received new orders in the block.
    // Match and clear the orders for each of them.
    // TODO: spawn a thread for each pair to process them in parallel.
    for (base_denom, quote_denom) in pairs {
        clear_orders_of_pair(
            ctx.storage,
            ctx.block.height,
            app_cfg.addresses.dex,
            &mut oracle_querier,
            &mut account_querier,
            app_cfg.maker_fee_rate.into_inner(),
            app_cfg.taker_fee_rate.into_inner(),
            base_denom,
            quote_denom,
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
        .may_add_message(if !refunds.is_empty() {
            Some(Message::batch_transfer(refunds)?)
        } else {
            None
        })
        .may_add_message(if !fee_payments.is_empty() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: fee_payments,
                },
                fees,
            )?)
        } else {
            None
        })
        .add_events(events)?)
}

#[inline]
fn clear_orders_of_pair(
    storage: &mut dyn Storage,
    current_block_height: u64,
    dex_addr: Addr,
    oracle_querier: &mut OracleQuerier,
    account_querier: &mut AccountQuerier,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    base_denom: Denom,
    quote_denom: Denom,
    events: &mut EventBuilder,
    refunds: &mut BTreeMap<Addr, Coins>,
    fees: &mut Coins,
    fee_payments: &mut BTreeMap<Addr, Coins>,
    volumes: &mut HashMap<Addr, Uint128>,
    volumes_by_username: &mut HashMap<Username, Uint128>,
) -> anyhow::Result<()> {
    // Create iterators over user orders.
    //
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let bid_iter = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending);
    let ask_iter = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending);

    // Create iterators over passive orders.
    //
    // If the pool doesn't have passive liquidity (reserve is `None`), simply
    // use empty iterators.
    let reserve = RESERVES.may_load(storage, (&base_denom, &quote_denom))?;
    let (passive_bid_iter, passive_ask_iter) = match &reserve {
        Some(reserve) => {
            // Create the passive liquidity orders if the pair has a pool.
            let pair = PAIRS.load(storage, (&base_denom, &quote_denom))?;
            pair.reflect_curve(base_denom.clone(), quote_denom.clone(), reserve)?
        },
        None => (Box::new(iter::empty()) as _, Box::new(iter::empty()) as _),
    };

    // Merge the orders from users and from the passive pool.
    let merged_bid_iter = MergedOrders::new(
        bid_iter,
        passive_bid_iter,
        IterationOrder::Descending,
        dex_addr,
    );
    let merged_ask_iter = MergedOrders::new(
        ask_iter,
        passive_ask_iter,
        IterationOrder::Ascending,
        dex_addr,
    );

    // Run the order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_orders(merged_bid_iter, merged_ask_iter)?;

    // If no matching orders were found, then we're done with this pair.
    // Continue to the next pair.
    let Some((lower_price, higher_price)) = range else {
        return Ok(());
    };

    // Choose the clearing price. Any price within `range` gives the same
    // volume (measured in the base asset). We can either take
    //
    // - the lower end,
    // - the higher end, or
    // - the midpoint of the range.
    //
    // Here we choose the midpoint.
    let clearing_price = lower_price.checked_add(higher_price)?.checked_mul(HALF)?;

    events.push(OrdersMatched {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        clearing_price,
        volume,
    })?;

    let mut inflows = Coins::new();
    let mut outflows = Coins::new();

    // Handle order filling outcomes for the user placed orders.
    for FillingOutcome {
        order_direction,
        order_price,
        order_id,
        order,
        filled,
        cleared,
        refund_base,
        refund_quote,
        fee_base,
        fee_quote,
    } in fill_orders(
        bids,
        asks,
        clearing_price,
        volume,
        current_block_height,
        maker_fee_rate,
        taker_fee_rate,
    )? {
        update_trading_volumes(
            storage,
            oracle_querier,
            account_querier,
            &base_denom,
            filled,
            order.user,
            volumes,
            volumes_by_username,
        )?;

        if order.user == dex_addr {
            // The order only exists in the storage if it's not owned by the dex, since
            // the passive orders are "virtual". If it is virtual, we need to update the
            // reserve.
            match order_direction {
                Direction::Bid => {
                    inflows.insert(Coin {
                        denom: base_denom.clone(),
                        amount: filled,
                    })?;
                    outflows.insert(Coin {
                        denom: quote_denom.clone(),
                        amount: filled.checked_mul_dec_floor(clearing_price)?,
                    })?;
                },
                Direction::Ask => {
                    inflows.insert(Coin {
                        denom: quote_denom.clone(),
                        amount: refund_quote,
                    })?;
                    outflows.insert(Coin {
                        denom: base_denom.clone(),
                        amount: filled,
                    })?;
                },
            }
        } else {
            // Add refund to the refunds map
            let refund = Coins::try_from([
                Coin {
                    denom: base_denom.clone(),
                    amount: refund_base,
                },
                Coin {
                    denom: quote_denom.clone(),
                    amount: refund_quote,
                },
            ])?;

            refunds
                .entry(order.user)
                .or_default()
                .insert_many(refund.clone())?;

            // Handle fees.
            if fee_base.is_non_zero() {
                fees.insert(Coin::new(base_denom.clone(), fee_base)?)?;
                fee_payments
                    .entry(order.user)
                    .or_default()
                    .insert(Coin::new(base_denom.clone(), fee_base)?)?;
            }

            if fee_quote.is_non_zero() {
                fees.insert(Coin::new(quote_denom.clone(), fee_quote)?)?;
                fee_payments
                    .entry(order.user)
                    .or_default()
                    .insert(Coin::new(quote_denom.clone(), fee_quote)?)?;
            }

            // Include fee information in the event
            let fee = if fee_base.is_non_zero() {
                Some(Coin {
                    denom: base_denom.clone(),
                    amount: fee_base,
                })
            } else if fee_quote.is_non_zero() {
                Some(Coin {
                    denom: quote_denom.clone(),
                    amount: fee_quote,
                })
            } else {
                None
            };

            // Emit event for filled user orders to be used by the frontend
            events.push(OrderFilled {
                user: order.user,
                order_id,
                clearing_price,
                filled,
                refund,
                fee,
                cleared,
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
                direction: order_direction,
            })?;

            if cleared {
                // Remove the order from the storage if it was fully filled
                LIMIT_ORDERS.remove(
                    storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        order_direction,
                        order_price,
                        order_id,
                    ),
                )?;
            } else {
                LIMIT_ORDERS.save(
                    storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        order_direction,
                        order_price,
                        order_id,
                    ),
                    &order,
                )?;
            }
        }
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

/// Updates trading volumes for both user addresses and usernames
fn update_trading_volumes(
    storage: &mut dyn Storage,
    oracle_querier: &mut OracleQuerier,
    account_querier: &mut AccountQuerier,
    base_denom: &Denom,
    filled: Uint128,
    order_user: Addr,
    volumes: &mut HashMap<Addr, Uint128>,
    volumes_by_username: &mut HashMap<Username, Uint128>,
) -> anyhow::Result<()> {
    // Calculate the vo lume in USD for the filled order
    let base_asset_price = oracle_querier.query_price(base_denom, None)?;
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
