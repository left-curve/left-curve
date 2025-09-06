use {
    crate::{
        MAX_ORACLE_STALENESS, NEXT_ORDER_ID, ORDERS, PAIRS, PAUSED, RESERVES, RESTING_ORDER_BOOK,
        VOLUMES, VOLUMES_BY_USER,
        core::{FillingOutcome, MatchingOutcome, PassiveLiquidityPool, fill_orders, match_orders},
        liquidity_depth::{decrease_liquidity_depths, increase_liquidity_depths},
    },
    dango_account_factory::AccountQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{
            CallbackMsg, Direction, ExecuteMsg, Order, OrderCanceled, OrderFilled, OrdersMatched,
            Paused, ReplyMsg, RestingOrderBookState, TimeInForce,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, DecCoins, Denom, EventBuilder, Inner, IsZero, Message, MultiplyFraction,
        MutableCtx, NonZero, Number, NumberConst, Order as IterationOrder, Response, StdError,
        StdResult, Storage, SubMessage, SubMsgResult, SudoCtx, TransferBuilder, Udec128, Udec128_6,
        Udec128_24,
    },
    std::collections::{BTreeMap, BTreeSet, HashMap, hash_map::Entry},
};

const HALF: Udec128 = Udec128::new_percent(50);

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

    // Load all existing pairs and their parameters.
    let pairs = PAIRS
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    // Delete the passive orders left over from the previous block.
    for ((denoms, direction, price, order_id), order) in ORDERS
        .idx
        .user
        .prefix(app_cfg.addresses.dex)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        decrease_liquidity_depths(
            ctx.storage,
            &denoms.0,
            &denoms.1,
            direction,
            price,
            order.remaining,
            &pairs[&denoms].bucket_sizes,
        )?;

        ORDERS.remove(ctx.storage, (denoms, direction, price, order_id))?;
    }

    // Loop through all trading pairs. Match and clear the orders for each of them.
    // TODO: spawn a thread for each pair to process them in parallel.
    for (denoms, pair) in pairs {
        clear_orders_of_pair(
            ctx.storage,
            ctx.block.height,
            app_cfg.addresses.dex,
            &mut oracle_querier,
            &mut account_querier,
            app_cfg.maker_fee_rate.into_inner(),
            app_cfg.taker_fee_rate.into_inner(),
            denoms.0,
            denoms.1,
            &pair.bucket_sizes,
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
    current_block_height: u64,
    dex_addr: Addr,
    oracle_querier: &mut OracleQuerier,
    account_querier: &mut AccountQuerier,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    base_denom: Denom,
    quote_denom: Denom,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
    events: &mut EventBuilder,
    refunds: &mut TransferBuilder<DecCoins<6>>,
    fees: &mut DecCoins<6>,
    fee_payments: &mut TransferBuilder<DecCoins<6>>,
    volumes: &mut HashMap<Addr, Udec128_6>,
    volumes_by_username: &mut HashMap<Username, Udec128_6>,
) -> anyhow::Result<()> {
    // --------------------- 1. Update passive pool orders ---------------------

    // Generate updated passive orders and insert them into the book.
    //
    // NOTE: This operation assumes the pool only generates a limited number of
    // orders. The admin should set proper parameters so that the pool doesn't
    // generate too many orders.
    if let Some(reserve) = RESERVES.may_load(storage, (&base_denom, &quote_denom))? {
        let pair = PAIRS.load(storage, (&base_denom, &quote_denom))?;
        match pair.reflect_curve(
            oracle_querier,
            base_denom.clone(),
            quote_denom.clone(),
            &reserve,
        ) {
            Ok((passive_bids, passive_asks)) => {
                for (price, amount) in passive_bids {
                    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;
                    order_id = !order_id; // See the docs of the `OrderId` type on why we invert this.

                    let remaining = amount.checked_into_dec()?;

                    increase_liquidity_depths(
                        storage,
                        &base_denom,
                        &quote_denom,
                        Direction::Bid,
                        price,
                        remaining,
                        bucket_sizes,
                    )?;

                    ORDERS.save(
                        storage,
                        (
                            (base_denom.clone(), quote_denom.clone()),
                            Direction::Bid,
                            price,
                            order_id,
                        ),
                        &Order {
                            user: dex_addr,
                            id: order_id,
                            direction: Direction::Bid,
                            time_in_force: TimeInForce::GoodTilCanceled,
                            price,
                            amount,
                            remaining,
                            created_at_block_height: None,
                        },
                    )?;
                }

                for (price, amount) in passive_asks {
                    let (order_id, _) = NEXT_ORDER_ID.increment(storage)?;
                    let remaining = amount.checked_into_dec()?;

                    increase_liquidity_depths(
                        storage,
                        &base_denom,
                        &quote_denom,
                        Direction::Ask,
                        price,
                        remaining,
                        bucket_sizes,
                    )?;

                    ORDERS.save(
                        storage,
                        (
                            (base_denom.clone(), quote_denom.clone()),
                            Direction::Ask,
                            price,
                            order_id,
                        ),
                        &Order {
                            user: dex_addr,
                            id: order_id,
                            direction: Direction::Ask,
                            time_in_force: TimeInForce::GoodTilCanceled,
                            price,
                            amount,
                            remaining,
                            created_at_block_height: None,
                        },
                    )?;
                }
            },
            // If there is an error, we simply emit a tracing log and move on.
            // Do not bail.
            // The most common cause of errors here is oracle downtime (that
            // exceeds the `MAX_STALENESS` in `OracleQuerier`), which isn't a
            // fatal error that necessitates halting of trading.
            Err(_err) => {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    %base_denom,
                    %quote_denom,
                    ?reserve,
                    %_err,
                    "!!! REFLECT CURVE FAILED !!!"
                );
            },
        }
    }

    // ----------------------- 2. Perform order matching -----------------------

    // Create iterators over orders.
    //
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let mut bid_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending)
        .map(|res| {
            let ((price, _order_id), order) = res?;
            Ok((price, order))
        });
    let mut ask_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((price, _order_id), order) = res?;
            Ok((price, order))
        });

    // Run the limit order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_orders(&mut bid_iter, &mut ask_iter)?;

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

    // Drop the iterators, which hold immutable references to the storage,
    // so that we can write to storage later.
    // The Rust compiler isn't smart enough to do this on its own.
    drop(bid_iter);
    drop(ask_iter);

    // ----------------------- 3. Perform order filling ------------------------

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

    // ------------------------ 4. Handle filled orders ------------------------

    // In the previous step, we ran the order filling algorithm. However, the
    // algorithm is a pure function with no side effects. Now, we must execute
    // the desired side effects:
    // - update order and reserve status in the contract store;
    // - refund appropriate amounts of tokens to users;
    // - emit events.

    // Track the inflows and outflows of the dex.
    let mut inflows = DecCoins::new();
    let mut outflows = DecCoins::new();

    // Handle order filling outcomes for the user placed orders.
    for FillingOutcome {
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
        if order.user != dex_addr {
            fill_user_order(
                order.user,
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

            // For limit orders, delete it from storage if fully filled, or
            // update if partially filled.
            // For market orders, delete it from storage, and refund the user
            // the remaining amount, as market orders are immediate-or-cancel.
            match order.time_in_force {
                TimeInForce::GoodTilCanceled => {
                    decrease_liquidity_depths(
                        storage,
                        &base_denom,
                        &quote_denom,
                        order.direction,
                        order.price,
                        filled_base,
                        bucket_sizes,
                    )?;

                    if order.remaining.is_zero() {
                        ORDERS.remove(
                            storage,
                            (
                                (base_denom.clone(), quote_denom.clone()),
                                order.direction,
                                order.price,
                                order.id,
                            ),
                        )?;
                    } else {
                        ORDERS.save(
                            storage,
                            (
                                (base_denom.clone(), quote_denom.clone()),
                                order.direction,
                                order.price,
                                order.id,
                            ),
                            &order,
                        )?;
                    }
                },
                TimeInForce::ImmediateOrCancel => {
                    refund_order(&base_denom, &quote_denom, order, events, refunds)?;

                    ORDERS.remove(
                        storage,
                        (
                            (base_denom.clone(), quote_denom.clone()),
                            order.direction,
                            order.price,
                            order.id,
                        ),
                    )?;
                },
            }
        } else {
            fill_passive_order(
                &base_denom,
                &quote_denom,
                order.direction,
                filled_base,
                filled_quote,
                &mut inflows,
                &mut outflows,
            )?;
        };

        // Emit event for filled orders to be used by the frontend.
        events.push(OrderFilled {
            user: order.user,
            id: order.id,
            time_in_force: order.time_in_force,
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            direction: order.direction,
            filled_base,
            filled_quote,
            refund_base,
            refund_quote,
            fee_base,
            fee_quote,
            clearing_price,
            cleared: order.remaining.is_zero(),
        })?;

        // Record the order's trading volume.
        update_trading_volumes(
            storage,
            oracle_querier,
            account_querier,
            &base_denom,
            filled_base,
            order.user,
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

    // ------------------------- 5. Cancel IOC orders --------------------------

    for order in ORDERS
        .idx
        .time_in_force
        .prefix(TimeInForce::ImmediateOrCancel)
        .append((base_denom.clone(), quote_denom.clone()))
        .values(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        ORDERS.remove(
            storage,
            (
                (base_denom.clone(), quote_denom.clone()),
                order.direction,
                order.price,
                order.id,
            ),
        )?;

        refund_order(&base_denom, &quote_denom, order, events, refunds)?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Canceled IOC orders"
        );
    }

    // ----------------- 6. Save the resting order book state ------------------

    // Find the best bid and ask prices that remains after all the previous steps.
    let best_bid_price = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .keys(storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .map(|(price, _order_id)| price);
    let best_ask_price = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .keys(storage, None, None, IterationOrder::Ascending)
        .next()
        .transpose()?
        .map(|(price, _order_id)| price);

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

/// Handle the `FillingOutcome` of a user order (limit or market).
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

/// Add a refund to the `refunds` transfer builder and emit an order canceled
/// event of a market order.
///
/// If the order is not a market order, then this function is a no-op.
fn refund_order(
    base_denom: &Denom,
    quote_denom: &Denom,
    order: Order,
    events: &mut EventBuilder,
    refunds: &mut TransferBuilder<DecCoins<6>>,
) -> StdResult<()> {
    // This function is only intended to be called to when IOC orders are to be
    // automatically canceled.
    debug_assert_eq!(
        order.time_in_force,
        TimeInForce::ImmediateOrCancel,
        "`refund_order` function called for an order that isn't IOC: {order:?}"
    );

    let (refund_denom, refund_amount) = match order.direction {
        Direction::Bid => {
            let remaining_in_quote = order.remaining.checked_mul_dec_floor(order.price)?;
            (quote_denom.clone(), remaining_in_quote)
        },
        Direction::Ask => (base_denom.clone(), order.remaining),
    };

    if refund_amount.is_non_zero() {
        events.push(OrderCanceled {
            user: order.user,
            id: order.id,
            time_in_force: order.time_in_force,
            remaining: order.remaining,
            refund: (refund_denom.clone(), refund_amount).into(),
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            direction: order.direction,
            price: order.price,
            amount: order.amount,
        })?;

        refunds.insert(order.user, refund_denom, refund_amount)?;
    }

    Ok(())
}

/// Updates trading volumes for both user addresses and usernames
fn update_trading_volumes(
    storage: &mut dyn Storage,
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
        Err(_err) => {
            #[cfg(feature = "tracing")]
            {
                tracing::warn!(
                    %base_denom,
                    %_err,
                    "Failed to query oracle price for base asset. Skipping volume update"
                );
            }

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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            config::{AppAddresses, AppConfig},
            constants::{dango, usdc},
            dex::{Geometric, OrderId, PairParams, PassiveLiquidity, Price},
            oracle::PriceSource,
        },
        grug::{Bounded, MockContext, MockQuerier, Timestamp, Uint128},
        std::str::FromStr,
        test_case::test_case,
    };

    const MOCK_USER: Addr = Addr::mock(123);
    const MOCK_DEX: Addr = Addr::mock(0);
    const MOCK_ORACLE: Addr = Addr::mock(1);
    const MOCK_ACCOUNT_FACTORY: Addr = Addr::mock(2);
    const MOCK_BLOCK_HEIGHT: u64 = 888;
    const MOCK_BLOCK_TIMESTAMP: Timestamp = Timestamp::from_seconds(1_000_000);

    #[test_case(
        vec![],
        vec![]
        => RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: None,
            mid_price: None,
        };
        "no orders"
    )]
    // The ask partially consumes the bid.
    // The bid remains the best price in the book.
    #[test_case(
        vec![],
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(2)),
            (Direction::Ask, Price::new(50), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: Some(Price::new(50)),
            best_ask_price: None,
            mid_price: Some(Price::new(50)),
        };
        "limit bid partially matched"
    )]
    // Same as the previous test, but the best bid that got partially matched is
    // a market order.
    // Market orders are immediate-or-cancel, so the best price falls back to
    // the next best limit or passive bid, which is 49.
    #[test_case(
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(2)),
        ],
        vec![
            (Direction::Ask, Price::new(50), Uint128::new(1)),
            (Direction::Bid, Price::new(49), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: Some(Price::new(49)),
            best_ask_price: None,
            mid_price: Some(Price::new(49)),
        };
        "market bid partially matched, limit bid other unmatched"
    )]
    // The bid and ask at 50 consumes each other exactly.
    // The bid at 49 becomes the best price remaining in the book.
    #[test_case(
        vec![],
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(50), Uint128::new(1)),
            (Direction::Bid, Price::new(49), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: Some(Price::new(49)),
            best_ask_price: None,
            mid_price: Some(Price::new(49)),
        };
        "limit bid first unmatched"
    )]
    // Same as the previous test, but the order at 49 is a market order.
    // The best price falls back to the limit bid at 49.
    #[test_case(
        vec![
            (Direction::Bid, Price::new(49), Uint128::new(1)),
        ],
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(50), Uint128::new(1)),
            (Direction::Bid, Price::new(48), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: Some(Price::new(48)),
            best_ask_price: None,
            mid_price: Some(Price::new(48)),
        };
        "market bid first unmatched, limit bid other unmatched"
    )]
    // The following test cases are the same as above, except for mirrored to
    // the ask side of the book.
    #[test_case(
        vec![],
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(50), Uint128::new(2)),
        ]
        => RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: Some(Price::new(50)),
            mid_price: Some(Price::new(50)),
        };
        "limit ask partially matched"
    )]
    #[test_case(
        vec![
            (Direction::Ask, Price::new(50), Uint128::new(2)),
        ],
        vec![
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(51), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: Some(Price::new(51)),
            mid_price: Some(Price::new(51)),
        };
        "market ask partially matched, limit ask other unmatched"
    )]
    #[test_case(
        vec![],
        vec![
            (Direction::Ask, Price::new(50), Uint128::new(1)),
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(51), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: Some(Price::new(51)),
            mid_price: Some(Price::new(51)),
        };
        "limit ask first unmatched"
    )]
    #[test_case(
        vec![
            (Direction::Ask, Price::new(51), Uint128::new(1)),
        ],
        vec![
            (Direction::Ask, Price::new(50), Uint128::new(1)),
            (Direction::Bid, Price::new(50), Uint128::new(1)),
            (Direction::Ask, Price::new(52), Uint128::new(1)),
        ]
        => RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: Some(Price::new(52)),
            mid_price: Some(Price::new(52)),
        };
        "market ask first unmatched, limit ask other unmatched"
    )]
    fn properly_determine_resting_order_book_state(
        market_orders: Vec<(Direction, Price, Uint128)>, // direction, price, amount
        limit_orders: Vec<(Direction, Price, Uint128)>,  // direction, price, amount
    ) -> RestingOrderBookState {
        let querier = MockQuerier::new()
            .with_raw_contract_storage(MOCK_ACCOUNT_FACTORY, |_storage| {
                // The `update_trading_volumes` function queries the username
                // associated with the user address.
                // Since trading volume is irrelevant for this test, we simply
                // do nothing, so that no username is found and the volume updating
                // logic is simply skipped.
            })
            .with_raw_contract_storage(MOCK_ORACLE, |storage| {
                // Set the prices for dango and USDC.
                // Used by the `update_trading_volumes` function.
                dango_oracle::PRICE_SOURCES
                    .save(
                        storage,
                        &dango::DENOM,
                        &PriceSource::Fixed {
                            humanized_price: Udec128::new(50),
                            precision: 0,
                            timestamp: MOCK_BLOCK_TIMESTAMP,
                        },
                    )
                    .unwrap();
                dango_oracle::PRICE_SOURCES
                    .save(
                        storage,
                        &usdc::DENOM,
                        &PriceSource::Fixed {
                            humanized_price: Udec128::new(1),
                            precision: 6,
                            timestamp: MOCK_BLOCK_TIMESTAMP,
                        },
                    )
                    .unwrap();
            })
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: MOCK_ACCOUNT_FACTORY,
                    dex: MOCK_DEX,
                    oracle: MOCK_ORACLE,
                    ..Default::default()
                },
                maker_fee_rate: Bounded::new_unchecked(Udec128::ZERO), // set fee rates to zero for simplicity
                taker_fee_rate: Bounded::new_unchecked(Udec128::ZERO),
                ..Default::default()
            })
            .unwrap();
        let mut ctx = MockContext::new()
            .with_querier(querier)
            .with_block_height(MOCK_BLOCK_HEIGHT)
            .with_sender(MOCK_DEX)
            .with_funds(Coins::new());
        let market_order_count = market_orders.len();

        // Save paused state as false.
        PAUSED.save(&mut ctx.storage, &false).unwrap();

        // Save pair config.
        PAIRS
            .save(
                &mut ctx.storage,
                (&dango::DENOM, &usdc::DENOM),
                &PairParams {
                    lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                    pool_type: PassiveLiquidity::Geometric(Geometric {
                        spacing: Udec128::ZERO,
                        ratio: Bounded::new_unchecked(Udec128::ONE),
                        limit: 10,
                    }),
                    bucket_sizes: BTreeSet::new(),
                    swap_fee_rate: Bounded::new_unchecked(Udec128::from_str("0.001").unwrap()),
                    min_order_size: Uint128::ZERO,
                },
            )
            .unwrap();

        // Save the market orders.
        for (index, (direction, price, amount)) in market_orders.into_iter().enumerate() {
            let id = OrderId::new(index as _);
            ORDERS
                .save(
                    &mut ctx.storage,
                    (
                        (dango::DENOM.clone(), usdc::DENOM.clone()),
                        direction,
                        price,
                        id,
                    ),
                    &Order {
                        user: MOCK_USER,
                        id,
                        direction,
                        time_in_force: TimeInForce::ImmediateOrCancel,
                        price,
                        amount,
                        remaining: amount.checked_into_dec().unwrap(),
                        created_at_block_height: Some(MOCK_BLOCK_HEIGHT),
                    },
                )
                .unwrap();
        }

        // Save the limit orders.
        for (index, (direction, price, amount)) in limit_orders.into_iter().enumerate() {
            let id = OrderId::new((market_order_count + index) as _);
            ORDERS
                .save(
                    &mut ctx.storage,
                    (
                        (dango::DENOM.clone(), usdc::DENOM.clone()),
                        direction,
                        price,
                        id,
                    ),
                    &Order {
                        user: MOCK_USER,
                        id,
                        direction,
                        time_in_force: TimeInForce::GoodTilCanceled,
                        price,
                        amount,
                        remaining: amount.checked_into_dec().unwrap(),
                        created_at_block_height: Some(MOCK_BLOCK_HEIGHT),
                    },
                )
                .unwrap();
        }

        // Run the auction.
        auction(ctx.as_mutable()).unwrap();

        // Return the resting order book state after the auction to check for accuracy.
        RESTING_ORDER_BOOK
            .load(&ctx.storage, (&dango::DENOM, &usdc::DENOM))
            .unwrap()
    }
}
