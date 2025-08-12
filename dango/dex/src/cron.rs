use {
    crate::{
        LIMIT_ORDERS, MARKET_ORDERS, MAX_ORACLE_STALENESS, PAIRS, PAUSED, RESERVES,
        RESTING_ORDER_BOOK, VOLUMES, VOLUMES_BY_USER,
        core::{
            FillingOutcome, MatchingOutcome, MergedOrders, PassiveLiquidityPool, fill_orders,
            match_orders,
        },
    },
    dango_account_factory::AccountQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{
            CallbackMsg, Direction, ExecuteMsg, LimitOrder, MarketOrder, Order, OrderFilled,
            OrderId, OrderTrait, OrdersMatched, PassiveOrder, Paused, Price, ReplyMsg,
            RestingOrderBookState,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Api, Coins, DecCoins, Denom, EventBuilder, Inner, IsZero, Message, MultiplyFraction,
        MutableCtx, Number, NumberConst, Order as IterationOrder, Response, StdError, StdResult,
        Storage, SubMessage, SubMsgResult, SudoCtx, TransferBuilder, Udec128, Udec128_6,
        Udec128_24,
    },
    std::{
        collections::{BTreeMap, HashMap, hash_map::Entry},
        iter,
    },
};

const HALF: Udec128 = Udec128::new_percent(50);

type MarketOrders = BTreeMap<(Price, OrderId), MarketOrder>;

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    // Skip the auction if trading is paused.
    if PAUSED.load(ctx.storage)? {
        return Ok(Response::new());
    }

    // Use submessage "reply on error" to catch errors that may happen during
    // the auction.
    Ok(Response::new().add_submessage(SubMessage::reply_on_error(
        Message::execute(
            ctx.contract,
            &ExecuteMsg::Callback(CallbackMsg::Auction {}),
            Coins::new(),
        )?,
        &ReplyMsg::AfterAuction {},
    )?))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn reply(ctx: SudoCtx, msg: ReplyMsg, res: SubMsgResult) -> StdResult<Response> {
    match msg {
        ReplyMsg::AfterAuction {} => {
            let error = res.unwrap_err(); // safe to unwrap because we only request reply on error

            #[cfg(feature = "library")]
            {
                tracing::error!(error, "!!! AUCTION FAILED !!!");
            }

            // Pause trading in case of a failure.
            PAUSED.save(ctx.storage, &true)?;

            Response::new().add_event(Paused { error: Some(error) })
        },
    }
}

pub(crate) fn auction(ctx: MutableCtx) -> anyhow::Result<Response> {
    let app_cfg = ctx.querier.query_dango_config()?;

    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);
    let mut account_querier = AccountQuerier::new(app_cfg.addresses.account_factory, ctx.querier);

    let mut events = EventBuilder::new();
    let mut refunds = TransferBuilder::<DecCoins<6>>::new();
    let mut volumes = HashMap::<Addr, Udec128_6>::new();
    let mut volumes_by_username = HashMap::<Username, Udec128_6>::new();
    let mut fees = DecCoins::<6>::new();
    let mut fee_payments = TransferBuilder::<DecCoins<6>>::new();

    // Collect all market orders received during this block.
    let mut market_orders = MARKET_ORDERS
        .values(ctx.storage, None, None, IterationOrder::Ascending)
        .try_fold(BTreeMap::new(), |mut acc, res| {
            let ((pair, direction, price, order_id), order) = res?;
            let (bids, asks): &mut (MarketOrders, MarketOrders) = acc.entry(pair).or_default();
            match direction {
                Direction::Bid => {
                    bids.insert((price, order_id), order);
                },
                Direction::Ask => {
                    asks.insert((price, order_id), order);
                },
            }

            Ok::<_, StdError>(acc)
        })?;

    // Since market orders are immediate-or-cancel, delete them from storage.
    MARKET_ORDERS.clear(ctx.storage, None, None);

    // Loop through all trading pairs. Match and clear the orders for each of them.
    // TODO: only process pairs that have received new orders during this block.
    // TODO: spawn a thread for each pair to process them in parallel.
    for (base_denom, quote_denom) in PAIRS
        .keys(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        let pair = (base_denom.clone(), quote_denom.clone());
        let (market_bids, market_asks) = market_orders.remove(&pair).unwrap_or_default();

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
            market_bids,
            market_asks,
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

    // Round refunds and fee to integer amounts. Round _down_ in both cases.
    let refunds = refunds.into_batch();
    let fees = fees.into_coins_floor();

    Ok(Response::new()
        .may_add_message(if !refunds.is_empty() {
            Some(Message::Transfer(refunds))
        } else {
            None
        })
        .may_add_message(if fees.is_non_empty() {
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
    market_bids: MarketOrders,
    market_asks: MarketOrders,
    events: &mut EventBuilder,
    refunds: &mut TransferBuilder<DecCoins<6>>,
    fees: &mut DecCoins<6>,
    fee_payments: &mut TransferBuilder<DecCoins<6>>,
    volumes: &mut HashMap<Addr, Udec128_6>,
    volumes_by_username: &mut HashMap<Username, Udec128_6>,
) -> anyhow::Result<()> {
    // ------------------------- 1. Prepare iterators --------------------------

    // Create iterators over the limit orders.
    //
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let limit_bids = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending);
    let limit_asks = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending);

    // Create iterators over passive orders.
    //
    // If the pool doesn't have passive liquidity (reserve is `None`), or if
    // the order book reflection fails, simply use empty iterators. I.e. place
    // no passive liquidity orders.
    let reserve = RESERVES.may_load(storage, (&base_denom, &quote_denom))?;
    let (passive_bids, passive_asks) = match &reserve {
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
                let msg = format!("ERROR: reflect curve failed! base denom: {base_denom}, quote denom: {quote_denom}, reserve: {reserve:?}, error: {err}"); // TODO: use tracing instead
                api.debug(dex_addr, &msg);
            })
            .unwrap_or_else(|_| (Box::new(iter::empty()) as _, Box::new(iter::empty()) as _))
        },
        None => (Box::new(iter::empty()) as _, Box::new(iter::empty()) as _),
    };

    // Merge orders using the iterator abstraction.
    //
    // Notes:
    // 1. Iterate from the best to worst price. Meaning, for bids, from the
    //    highest to the lowest (descending); for asks, from the lowest to the
    //    highest (ascending).
    // 2. The market order vectors are ordered ascendingly, so for bids we need
    //    to reverse it.
    let mut merged_bids = nested_merged_orders(
        limit_bids,
        market_bids.into_iter().rev(),
        passive_bids,
        IterationOrder::Descending,
    );
    let mut merged_asks = nested_merged_orders(
        limit_asks,
        market_asks.into_iter(),
        passive_asks,
        IterationOrder::Ascending,
    );

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Processing pair"
        );
    }

    // ---------------------------- 2. Match orders ----------------------------

    // Run the limit order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_orders(&mut merged_bids, &mut merged_asks)?;

    // Any order that isn't visited during `match_limit_orders` is unmatched.
    // They will be handled later in part (5) of this function.
    // Here we `disassemble`, with two purposes:
    // 1. drop the limit order iterators, so that the immutable reference on
    //    `storage` is released;
    // 2. get the unmatched market orders, so we can process their cancelation.
    // Limit orders are good-until-canceled, so no action is needed for them.
    let (_, unmatched_market_bids) = merged_bids.disassemble();
    let (_, unmatched_market_asks) = merged_asks.disassemble();

    #[cfg(feature = "tracing")]
    {
        let range_str = match range {
            Some((lower_price, upper_price)) => format!("{lower_price}-{upper_price}"),
            None => "None".to_string(),
        };

        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            range = range_str,
            volume = volume.to_string(),
            num_matched_bids = bids.len(),
            num_matched_asks = asks.len(),
            "Matched limit orders"
        );
    }

    // ----------------------- 3. Fulfill matched orders -----------------------

    // If matching orders were found, then we need to fill the orders. All orders
    // are filled at the clearing price.
    let filling_outcomes = if let Some((lower_price, upper_price)) = range {
        // Choose the clearing price, based on the mid price of the resting
        // order book:
        // - if mid price is within the range, then use the mid price;
        // - if mid price is bigger than the upper bound of the range, then use
        //   the upper bound;
        // - if mid price is smaller than the lower bound of the range, then use
        //   the lower bound.
        // - if the mid price doesn't exist, use the middle point of the range.
        let clearing_price = match RESTING_ORDER_BOOK
            .may_load(storage, (&base_denom, &quote_denom))?
            .and_then(|book| book.mid_price)
        {
            Some(mid_price) => {
                if mid_price < lower_price {
                    lower_price
                } else if mid_price > upper_price {
                    upper_price
                } else {
                    mid_price
                }
            },
            None => lower_price.checked_add(upper_price)?.checked_mul(HALF)?,
        };

        events.push(OrdersMatched {
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

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            num_filling_outcomes = filling_outcomes.len(),
            "Filled orders"
        );
    }

    // ------------------- 4. Handle unmatched market orders -------------------

    for res in unmatched_market_bids {
        let (_price, bid) = res?;
        match bid {
            // Market orders are immediate-or-cancel, so refund the user.
            Order::Market(order) => {
                let remaining_quote = order.remaining.checked_mul_dec_floor(order.price)?;
                refunds.insert(order.user, quote_denom.clone(), remaining_quote)?;
            },
            _ => unreachable!("encountered an unmatched bid that isn't a market order: {bid:?}"),
        }
    }

    for res in unmatched_market_asks {
        let (_price, ask) = res?;
        match ask {
            Order::Market(order) => {
                refunds.insert(order.user, base_denom.clone(), order.remaining)?;
            },
            _ => unreachable!("encountered an unmatched ask that isn't a market order: {ask:?}"),
        }
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Handled unmatched orders"
        );
    }

    // ------------------------ 5. Handle filled orders ------------------------

    // Track the inflows and outflows of the dex.
    let mut inflows = DecCoins::new();
    let mut outflows = DecCoins::new();

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
        clearing_price,
    } in filling_outcomes
    {
        let (user, id) = if let Some((order_id, user)) = order.id_and_user() {
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

            (user, Some(order_id))
        } else {
            fill_passive_order(
                &base_denom,
                &quote_denom,
                order_direction,
                filled_base,
                filled_quote,
                &mut inflows,
                &mut outflows,
            )?;

            (dex_addr, None)
        };

        // Emit event for filled orders to be used by the frontend.
        events.push(OrderFilled {
            user,
            id,
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
            cleared: order.remaining().is_zero(),
        })?;

        // Record the order's trading volume.
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

    // Update the pool reserve.
    if inflows.is_non_empty() || outflows.is_non_empty() {
        RESERVES.update(storage, (&base_denom, &quote_denom), |mut reserve| {
            for inflow in inflows.into_coins_floor() {
                reserve.checked_add(&inflow)?;
            }

            for outflow in outflows.into_coins_ceil() {
                reserve.checked_sub(&outflow)?;
            }

            Ok::<_, StdError>(reserve)
        })?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Handled filled orders"
        );
    }

    // ----------------- 6. Save the resting order book state ------------------

    // Find the best bid and ask prices available.
    let best_bid_price = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .keys(storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .map(|(_direction, price, _order_id)| price);
    let best_ask_price = LIMIT_ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .keys(storage, None, None, IterationOrder::Ascending)
        .next()
        .transpose()?
        .map(|(_direction, price, _order_id)| price);

    // Determine the mid price:
    // - if both best bid and ask prices exist, then take the average of them;
    // - if only one of them exists, then use that price;
    // - if none of them exists, then `None`.
    let mid_price = match (best_bid_price, best_ask_price) {
        (Some(bid), Some(ask)) => Some(bid.checked_add(ask)?.checked_mul(HALF)?),
        (Some(bid), None) => Some(bid),
        (None, Some(ask)) => Some(ask),
        (None, None) => None,
    };

    RESTING_ORDER_BOOK.save(
        storage,
        (&base_denom, &quote_denom),
        &RestingOrderBookState {
            best_bid_price,
            best_ask_price,
            mid_price,
        },
    )?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            ?best_bid_price,
            ?best_ask_price,
            ?mid_price,
            "Saved resting order book state"
        )
    }

    Ok(())
}

/// Merges three iterators over limit, market, ans passive orders into one iterator.
/// It achieves this by nesting two `MergedOrders` iterators.
fn nested_merged_orders<A, B, C>(
    limit: A,
    market: B,
    passive: C,
    iteration_order: IterationOrder,
) -> MergedOrders<
    MergedOrders<
        impl Iterator<Item = StdResult<(Udec128_24, Order)>>,
        impl Iterator<Item = StdResult<(Udec128_24, Order)>>,
    >,
    impl Iterator<Item = StdResult<(Udec128_24, Order)>>, /* TODO: simplify this return type once trait alias is stablized: https://doc.rust-lang.org/beta/unstable-book/language-features/trait-alias.html */
>
where
    A: Iterator<Item = StdResult<((Udec128_24, OrderId), LimitOrder)>>,
    B: Iterator<Item = ((Udec128_24, OrderId), MarketOrder)>,
    C: Iterator<Item = (Udec128_24, PassiveOrder)>,
{
    // Make the three iterators return the same item type, so they can be merged.
    let limit = limit.map(|res| res.map(|((price, _), order)| (price, Order::Limit(order))));
    let market = market.map(|((price, _), order)| Ok((price, Order::Market(order))));
    let passive = passive.map(|(price, order)| Ok((price, Order::Passive(order))));

    // Merge the three iterators.
    //
    // The ordering of the three iterators matters! For two reasons:
    // 1. In `MergedOrders::new(a, b, ..)`, `b` is prioritized.
    //    We use the following priority: market > limit > passive.
    // 2. We need to disassemble the `MergedOrders` later and retrieve the market
    //    order iterator. Note that `Peekable<T>` can't be disassembled to take
    //    out the inner `T`. So we need to make sure the market order iterator
    //    is only nested once, not twice.
    MergedOrders::new(
        MergedOrders::new(passive, limit, iteration_order),
        market,
        iteration_order,
    )
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
    refund_base: Udec128_6,
    refund_quote: Udec128_6,
    fee_base: Udec128_6,
    fee_quote: Udec128_6,
    refunds: &mut TransferBuilder<DecCoins<6>>,
    fees: &mut DecCoins<6>,
    fee_payments: &mut TransferBuilder<DecCoins<6>>,
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
    filled_base: Udec128_6,
    filled_quote: Udec128_6,
    inflows: &mut DecCoins<6>,
    outflows: &mut DecCoins<6>,
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
    filled: Udec128_6,
    order_user: Addr,
    volumes: &mut HashMap<Addr, Udec128_6>,
    volumes_by_username: &mut HashMap<Username, Udec128_6>,
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
    let new_volume: Udec128_6 = base_asset_price.value_of_dec_amount(filled)?;

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
                    .unwrap_or(Udec128_6::ZERO)
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
                    .unwrap_or(Udec128_6::ZERO)
                    .checked_add(new_volume)?;

                v.insert(volume);
            },
        }
    }

    Ok(())
}
