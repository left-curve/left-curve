use {
    crate::{
        FillingOutcome, INCOMING_ORDERS, MatchingOutcome, NEXT_ORDER_ID, ORDERS, Order, PAIRS,
        PassiveLiquidityPool, RESERVES, core, fill_orders, match_orders,
    },
    anyhow::{anyhow, bail, ensure},
    dango_types::{
        bank,
        dex::{
            CreateLimitOrderRequest, Direction, ExecuteMsg, InstantiateMsg, LP_NAMESPACE,
            NAMESPACE, OrderCanceled, OrderFilled, OrderIds, OrderSubmitted, OrdersMatched, PairId,
            PairUpdate, PairUpdated, SwapExactAmountIn, SwapExactAmountOut,
        },
    },
    grug::{
        Addr, Coin, CoinPair, Coins, Denom, EventBuilder, GENESIS_SENDER, Inner, IsZero, Message,
        MultiplyFraction, MutableCtx, NonZero, Number, Order as IterationOrder, QuerierExt,
        Response, StdResult, Storage, SudoCtx, Udec128, Uint128, UniqueVec,
    },
    std::{
        cmp::Ordering,
        collections::{BTreeMap, BTreeSet},
        iter::Peekable,
    },
};

const HALF: Udec128 = Udec128::new_percent(50);

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    batch_update_pairs(ctx, msg.pairs)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::BatchUpdatePairs(updates) => batch_update_pairs(ctx, updates),
        ExecuteMsg::BatchUpdateOrders { creates, cancels } => {
            batch_update_orders(ctx, creates, cancels)
        },
        ExecuteMsg::ProvideLiquidity {
            base_denom,
            quote_denom,
        } => provide_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::WithdrawLiquidity {
            base_denom,
            quote_denom,
        } => withdraw_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::SwapExactAmountIn {
            route,
            minimum_output,
        } => swap_exact_amount_in(ctx, route.into_inner(), minimum_output),
        ExecuteMsg::SwapExactAmountOut { route, output } => {
            swap_exact_amount_out(ctx, route.into_inner(), output)
        },
    }
}

#[inline]
fn batch_update_pairs(ctx: MutableCtx, updates: Vec<PairUpdate>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()? || ctx.sender == GENESIS_SENDER,
        "only the owner can update a trading pair parameters"
    );

    let mut events = EventBuilder::with_capacity(updates.len());

    for update in updates {
        ensure!(
            update
                .params
                .lp_denom
                .starts_with(&[NAMESPACE.clone(), LP_NAMESPACE.clone()]),
            "LP token denom doesn't start with the correct prefix: `{}/{}/...`",
            NAMESPACE.as_ref(),
            LP_NAMESPACE.as_ref()
        );

        PAIRS.save(
            ctx.storage,
            (&update.base_denom, &update.quote_denom),
            &update.params,
        )?;

        events.push(PairUpdated {
            base_denom: update.base_denom,
            quote_denom: update.quote_denom,
        })?;
    }

    Ok(Response::new().add_events(events)?)
}

/// Internal use function for handling the logic of batch updating orders.
///
/// ## Inputs
///
/// - `storage`: Storage instance for read and write.
/// - `sender`:  The sender of the transaction. This is the address for whom to update the orders for.
/// - `creates`: The orders to create.
/// - `cancels`: The orders to cancel.
///
/// ## Outputs
///
/// - A `Coins` instance containing the deposits required to create the orders.
/// - A `Coins` instance containing the refunds for the cancelled orders.
/// - A vec of `ContractEvent` instances containing the events.
fn _batch_update_orders(
    storage: &mut dyn Storage,
    sender: Addr,
    creates: Vec<CreateLimitOrderRequest>,
    cancels: Option<OrderIds>,
) -> anyhow::Result<(Coins, Coins, EventBuilder)> {
    let mut deposits = Coins::new();
    let mut refunds = Coins::new();
    let mut events = EventBuilder::new();

    // --------------------------- 1. Cancel orders ----------------------------

    // First, collect all orders to be cancelled into memory.
    let orders_to_cancel = match cancels {
        // Cancel all orders.
        Some(OrderIds::All) => ORDERS
            .idx
            .user
            .prefix(sender)
            .range(storage, None, None, IterationOrder::Ascending)
            .map(|order| Ok((order?, false)))
            .chain(
                INCOMING_ORDERS
                    .prefix(sender)
                    .values(storage, None, None, IterationOrder::Ascending)
                    .map(|order| Ok((order?, true))),
            )
            .collect::<StdResult<Vec<_>>>()?,
        // Cancel selected orders.
        Some(OrderIds::Some(order_ids)) => order_ids
            .into_iter()
            .map(|order_id| {
                // First see if the order is the persistent storage. If not,
                // check the transient storage.
                if let Some(order) = ORDERS.idx.order_id.may_load(storage, order_id)? {
                    Ok((order, false))
                } else if let Some(order) = INCOMING_ORDERS.may_load(storage, (sender, order_id))? {
                    Ok((order, true))
                } else {
                    bail!("order with id `{order_id}` not found");
                }
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
        // Do nothing.
        None => Vec::new(),
    };

    // Now, cancel the orders one by one.
    for ((order_key, order), is_incoming) in orders_to_cancel {
        let ((base_denom, quote_denom), direction, price, order_id) = &order_key;

        ensure!(sender == order.user, "only the user can cancel the order");

        let refund = match direction {
            Direction::Bid => Coin {
                denom: quote_denom.clone(),
                amount: order.remaining.checked_mul_dec_floor(*price)?,
            },
            Direction::Ask => Coin {
                denom: base_denom.clone(),
                amount: order.remaining,
            },
        };

        refunds.insert(refund.clone())?;

        events.push(OrderCanceled {
            order_id: *order_id,
            remaining: order.remaining,
            refund,
        })?;

        if is_incoming {
            INCOMING_ORDERS.remove(storage, (sender, *order_id));
        } else {
            ORDERS.remove(storage, order_key)?;
        }
    }

    // --------------------------- 2. Create orders ----------------------------

    for order in creates {
        ensure!(
            PAIRS.has(storage, (&order.base_denom, &order.quote_denom)),
            "pair not found with base `{}` and quote `{}`",
            order.base_denom,
            order.quote_denom
        );

        let deposit = match order.direction {
            Direction::Bid => Coin {
                denom: order.quote_denom.clone(),
                amount: order.amount.checked_mul_dec_ceil(order.price)?,
            },
            Direction::Ask => Coin {
                denom: order.base_denom.clone(),
                amount: order.amount,
            },
        };

        let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

        // For BUY orders, invert the order ID. This is necessary for enforcing
        // price-time priority. See the docs on `OrderId` for details.
        if order.direction == Direction::Bid {
            order_id = !order_id;
        }

        deposits.insert(deposit.clone())?;

        events.push(OrderSubmitted {
            order_id,
            user: sender,
            base_denom: order.base_denom.clone(),
            quote_denom: order.quote_denom.clone(),
            direction: order.direction,
            price: order.price,
            amount: order.amount,
            deposit,
        })?;

        INCOMING_ORDERS.save(
            storage,
            (sender, order_id),
            &(
                (
                    (order.base_denom, order.quote_denom),
                    order.direction,
                    order.price,
                    order_id,
                ),
                Order {
                    user: sender,
                    amount: order.amount,
                    remaining: order.amount,
                },
            ),
        )?;
    }

    Ok((deposits, refunds, events))
}

#[inline]
fn batch_update_orders(
    mut ctx: MutableCtx,
    creates: Vec<CreateLimitOrderRequest>,
    cancels: Option<OrderIds>,
) -> anyhow::Result<Response> {
    let (deposits, refunds, events) =
        _batch_update_orders(ctx.storage, ctx.sender, creates, cancels)?;

    // Compute the amount of tokens that should be sent back to the users.
    //
    // This equals the amount that user has sent to the contract, plus the
    // amount that are to be refunded from the cancelled orders, and the amount
    // that the user is supposed to deposit for creating the new orders.
    ctx.funds
        .insert_many(refunds)?
        .deduct_many(deposits)
        .map_err(|e| anyhow!("insufficient funds for batch updating orders: {e}"))?;

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .add_events(events)?)
}

#[inline]
fn provide_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Get the deposited funds.
    let deposit = ctx
        .funds
        .take_pair((base_denom.clone(), quote_denom.clone()))?;

    // The user must have not sent any funds other the base/quote denoms.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}` and `{}`",
        ctx.funds,
        base_denom,
        quote_denom
    );

    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve. Default to empty if not found.
    let reserve = RESERVES
        .may_load(ctx.storage, (&base_denom, &quote_denom))?
        .map_or_else(
            || CoinPair::new_empty(base_denom.clone(), quote_denom.clone()),
            Ok,
        )?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Compute the amount of LP tokens to mint.
    let (reserve, lp_mint_amount) = pair.add_liquidity(reserve, lp_token_supply, deposit)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                denom: pair.lp_denom,
                amount: lp_mint_amount,
            },
            Coins::new(), // No funds needed for minting
        )?
    }))
    // TODO: add event
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
#[inline]
fn withdraw_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve.
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Get the sent LP tokens.
    let lp_burn_amount = ctx.funds.take(pair.lp_denom.clone()).amount;

    // The user must have not sent any funds other the LP token.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}`",
        ctx.funds,
        pair.lp_denom
    );

    // Calculate the amount of each asset to return
    let (reserve, refunds) = pair.remove_liquidity(reserve, lp_token_supply, lp_burn_amount)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    denom: pair.lp_denom,
                    amount: lp_burn_amount,
                },
                Coins::new(), // No funds needed for burning
            )?
        })
        .add_message(Message::transfer(ctx.sender, refunds)?))
    // TODO: add events
}

#[inline]
fn swap_exact_amount_in(
    ctx: MutableCtx,
    route: UniqueVec<PairId>,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let input = ctx.funds.into_one_coin()?;
    let (reserves, output) = core::swap_exact_amount_in(ctx.storage, route, input.clone())?;

    // Ensure the output is above the minimum.
    // If not minimum is specified, the output should at least be greater than zero.
    if let Some(minimum_output) = minimum_output {
        ensure!(
            output.amount >= minimum_output,
            "output amount is below the minimum: {} < {}",
            output.amount,
            minimum_output
        );
    } else {
        ensure!(output.amount.is_non_zero(), "output amount is zero");
    }

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, output.clone())?)
        .add_event(SwapExactAmountIn {
            user: ctx.sender,
            input,
            output,
        })?)
}

#[inline]
fn swap_exact_amount_out(
    mut ctx: MutableCtx,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<Response> {
    let (reserves, input) = core::swap_exact_amount_out(ctx.storage, route, output.clone())?;

    // The user must have sent no less than the required input amount.
    // Any extra is refunded.
    ctx.funds
        .insert(output.clone().into_inner())?
        .deduct(input.clone())
        .map_err(|e| anyhow!("insufficient input for swap: {e}"))?;

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    // Unlike `swap_exact_amount_in`, no need to check whether output is zero
    // here, because we already ensure it's non-zero.
    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .add_event(SwapExactAmountOut {
            user: ctx.sender,
            input,
            output: output.into_inner(),
        })?)
}

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let mut refunds = BTreeMap::new();
    let mut events = EventBuilder::new();

    // Collect incoming orders and clear the temporary storage.
    let incoming_orders = INCOMING_ORDERS.drain(ctx.storage, None, None)?;

    // Add incoming orders to the persistent storage.
    for (order_key, order) in incoming_orders.values() {
        ORDERS.save(ctx.storage, order_key.clone(), order)?;
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
        // Create the passive liquidity orders if the pair has a pool.
        if RESERVES.has(ctx.storage, (&base_denom, &quote_denom)) {
            let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;
            let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;
            let (passive_bids, passive_asks) =
                pair.reflect_curve(base_denom.clone(), quote_denom.clone(), &reserve)?;

            let passive_bids = passive_bids.into_iter().map(|(price, amount)| {
                Ok(((price, u64::MAX), Order {
                    user: ctx.contract,
                    amount,
                    remaining: amount,
                }))
            });
            let passive_asks = passive_asks.into_iter().map(|(price, amount)| {
                Ok(((price, 0), Order {
                    user: ctx.contract,
                    amount,
                    remaining: amount,
                }))
            });
            let mut reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;

            clear_orders_of_pair(
                ctx.storage,
                ctx.contract,
                base_denom.clone(),
                quote_denom.clone(),
                passive_bids,
                passive_asks,
                Some(&mut reserve),
                &mut events,
                &mut refunds,
            )?;

            RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;
        } else {
            clear_orders_of_pair(
                ctx.storage,
                ctx.contract,
                base_denom.clone(),
                quote_denom.clone(),
                vec![].into_iter(),
                vec![].into_iter(),
                None,
                &mut events,
                &mut refunds,
            )?;
        }
    }

    // Remove the dex from the refunds map since it cannot send tokens to itself.
    refunds.remove(&ctx.contract);

    Ok(Response::new()
        .may_add_message(if !refunds.is_empty() {
            Some(Message::batch_transfer(refunds)?)
        } else {
            None
        })
        .add_events(events)?)
}

struct MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    real: Peekable<A>,
    passive: Peekable<B>,
    order: grug::Order,
}

impl<A, B> MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    pub fn new(real: A, passive: B, order: grug::Order) -> Self {
        Self {
            real: real.peekable(),
            passive: passive.peekable(),
            order,
        }
    }
}

impl<A, B> Iterator for MergedOrders<A, B>
where
    A: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    B: Iterator<Item = StdResult<((Udec128, u64), Order)>>,
{
    type Item = StdResult<((Udec128, u64), Order)>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.real.peek(), self.passive.peek()) {
            (Some(Ok(((real_price, _), _))), Some(Ok(((passive_price, _), _)))) => {
                // Compare only the price since passive orders don't have an order ID.
                let ordering_raw = real_price.cmp(passive_price);
                let ordering = match self.order {
                    grug::Order::Ascending => ordering_raw,
                    grug::Order::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    Ordering::Less => self.real.next(),
                    // In case of equal price we give the passive liquidity priority.
                    _ => self.passive.next(),
                }
            },
            (Some(Ok(_)), None) => self.real.next(),
            (None, Some(Ok(_))) => self.passive.next(),
            (Some(Err(e)), _) => Some(Err(e.clone())),
            (_, Some(Err(e))) => Some(Err(e.clone())),
            (None, None) => None,
        }
    }
}

#[inline]
fn clear_orders_of_pair(
    storage: &mut dyn Storage,
    dex_addr: Addr,
    base_denom: Denom,
    quote_denom: Denom,
    passive_bids: impl Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    passive_asks: impl Iterator<Item = StdResult<((Udec128, u64), Order)>>,
    reserve: Option<&mut CoinPair>,
    events: &mut EventBuilder,
    refunds: &mut BTreeMap<Addr, Coins>,
) -> StdResult<()> {
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let bid_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending);
    let ask_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending);

    // Create the passive liquidity orders if the pair has a pool.
    let (merged_bid_iter, merged_ask_iter) = (
        MergedOrders::new(bid_iter, Box::new(passive_bids), grug::Order::Descending),
        MergedOrders::new(ask_iter, Box::new(passive_asks), grug::Order::Ascending),
    );

    let merged_bid_iter = merged_bid_iter.collect::<StdResult<Vec<_>>>()?;
    let merged_ask_iter = merged_ask_iter.collect::<StdResult<Vec<_>>>()?;

    println!("merged_bid_iter: {:?}", merged_bid_iter);
    println!("merged_ask_iter: {:?}", merged_ask_iter);

    // Run the order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_orders(
        merged_bid_iter.into_iter().map(|x| Ok(x)),
        merged_ask_iter.into_iter().map(|x| Ok(x)),
    )?;

    println!("bids: {:?}", bids);
    println!("asks: {:?}", asks);
    println!("range: {:?}", range);
    println!("volume: {:?}", volume);

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

    println!("clearing_price: {:?}", clearing_price);

    events.push(OrdersMatched {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        clearing_price,
        volume,
    })?;

    // Split fill_orders into two separate iterators depending on the order user
    let (dex_filling_outcomes, user_filling_outcomes): (Vec<_>, Vec<_>) =
        fill_orders(bids, asks, clearing_price, volume)?
            .into_iter()
            .partition(|x| x.order.user == dex_addr);

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
    } in user_filling_outcomes
    {
        println!("order: {:?}", order);
        println!("order_id: {:?}", order_id);
        println!("order_direction: {:?}", order_direction);
        println!("order_price: {:?}", order_price);
        println!("refund_base: {:?}", refund_base);
        println!("refund_quote: {:?}", refund_quote);
        println!("filled: {:?}", filled);
        println!("cleared: {:?}", cleared);

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

        events.push(OrderFilled {
            order_id,
            clearing_price,
            filled,
            refund: refund.clone(),
            fee: None,
            cleared,
        })?;

        // The order only exists in the storage if it's not owned by the dex, since
        // the passive orders are "virtual".
        refunds.entry(order.user).or_default().insert_many(refund)?;

        if cleared {
            ORDERS.remove(
                storage,
                (
                    (base_denom.clone(), quote_denom.clone()),
                    order_direction,
                    order_price,
                    order_id,
                ),
            )?;
        } else {
            println!("saving order: {:?}", order);
            ORDERS.save(
                storage,
                (
                    (base_denom.clone(), quote_denom.clone()),
                    order_direction,
                    order_price,
                    order_id,
                ),
                &order,
            )?;
            println!("post save");
        }
    }

    // Handle order filling outcomes for the matched orders belonging to the dex.
    if let Some(reserve) = reserve {
        for FillingOutcome {
            order_direction,
            filled,
            refund_quote,
            ..
        } in dex_filling_outcomes
        {
            match order_direction {
                Direction::Bid => {
                    reserve
                        .checked_add(&Coin::new(base_denom.clone(), filled)?)?
                        .checked_sub(&Coin::new(
                            quote_denom.clone(),
                            filled.checked_mul_dec_floor(clearing_price)?,
                        )?)?;
                },
                Direction::Ask => {
                    reserve
                        .checked_sub(&Coin::new(base_denom.clone(), filled)?)?
                        .checked_add(&Coin::new(quote_denom.clone(), refund_quote)?)?;
                },
            }
        }
    }

    Ok(())
}
