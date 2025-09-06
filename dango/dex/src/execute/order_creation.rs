use {
    crate::{
        NEXT_ORDER_ID, ORDERS, PAIRS, RESTING_ORDER_BOOK,
        liquidity_depth::increase_liquidity_depths,
    },
    anyhow::{anyhow, ensure},
    dango_types::dex::{CreateOrderRequest, Direction, Order, OrderCreated},
    grug::{
        Addr, Coin, Coins, EventBuilder, MultiplyFraction, Number, NumberConst, Storage, Udec128_24,
    },
};

pub(super) fn create_order(
    storage: &mut dyn Storage,
    current_block_height: u64,
    user: Addr,
    order: CreateOrderRequest,
    events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
    let pair = PAIRS
        .may_load(storage, (&order.base_denom, &order.quote_denom))?
        .ok_or_else(|| {
            anyhow!(
                "pair not found with base `{}` and quote `{}`",
                order.base_denom,
                order.quote_denom
            )
        })?;

    let amount_quote = order.amount.checked_mul_dec_ceil(*order.price)?;

    ensure!(
        amount_quote >= pair.min_order_size,
        "order size ({} {}) is less than the minimum ({} {})",
        amount_quote,
        order.quote_denom,
        pair.min_order_size,
        order.quote_denom
    );

    let deposit = match order.direction {
        Direction::Bid => Coin {
            denom: order.quote_denom.clone(),
            amount: amount_quote,
        },
        Direction::Ask => Coin {
            denom: order.base_denom.clone(),
            amount: *order.amount,
        },
    };

    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    // For BUY orders, invert the order ID. This is necessary for enforcing
    // price-time priority. See the docs on `OrderId` for details.
    if order.direction == Direction::Bid {
        order_id = !order_id;
    }

    events.push(OrderCreated {
        user,
        id: order_id,
        kind: OrderKind::Limit,
        base_denom: order.base_denom.clone(),
        quote_denom: order.quote_denom.clone(),
        direction: order.direction,
        price: Some(*order.price),
        amount: *order.amount,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    let remaining = order.amount.checked_into_dec()?;

    increase_liquidity_depths(
        storage,
        &order.base_denom,
        &order.quote_denom,
        order.direction,
        *order.price,
        remaining,
        &pair.bucket_sizes,
    )?;

    LIMIT_ORDERS.save(
        storage,
        (
            (order.base_denom, order.quote_denom),
            order.direction,
            *order.price,
            order_id,
        ),
        &Order {
            user,
            id: order_id,
            kind: OrderKind::Limit,
            price: *order.price,
            amount: *order.amount,
            remaining,
            created_at_block_height: Some(current_block_height),
        },
    )?;

    Ok(())
}

pub(super) fn create_market_order(
    storage: &mut dyn Storage,
    current_block_height: u64,
    user: Addr,
    order: CreateMarketOrderRequest,
    events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
    let pair = PAIRS
        .may_load(storage, (&order.base_denom, &order.quote_denom))?
        .ok_or_else(|| {
            anyhow!(
                "pair not found with base `{}` and quote `{}`",
                order.base_denom,
                order.quote_denom
            )
        })?;

    // Load the resting order book of the pair.
    // The best price available in the book, together with the order's maximum
    // slippage, will be used to determine the order's "limit price"
    let resting_order_book = RESTING_ORDER_BOOK
        .load(storage, (&order.base_denom, &order.quote_denom))
        .map_err(|err| {
            anyhow!(
                "can't create market order, because resting order book either doesn't exist or is corrupted. base denom: {}, quote denom: {}, err: {err}",
                order.base_denom,
                order.quote_denom
            )
        })?;

    let (price, amount_quote, deposit) = match order.direction {
        Direction::Bid => {
            let best_ask_price = resting_order_book.best_ask_price.ok_or_else(|| {
                anyhow!(
                    "can't create market bid order, because best ask price isn't available. base denom: {}, quote denom: {}",
                    order.base_denom,
                    order.quote_denom
                )
            })?;

            let one_add_max_slippage = Udec128_24::ONE.saturating_add(*order.max_slippage);
            let price = best_ask_price.saturating_mul(one_add_max_slippage);
            let amount_quote = order.amount.checked_mul_dec_ceil(price)?;

            (price, amount_quote, Coin {
                denom: order.quote_denom.clone(),
                amount: amount_quote,
            })
        },
        Direction::Ask => {
            let best_bid_price = resting_order_book.best_bid_price.ok_or_else(|| {
                anyhow!(
                    "can't create market ask order, because best bid price isn't available. base denom: {}, quote denom: {}",
                    order.base_denom,
                    order.quote_denom
                )
            })?;

            let one_sub_max_slippage = Udec128_24::ONE.saturating_sub(*order.max_slippage);
            let price = best_bid_price.saturating_mul(one_sub_max_slippage);
            let amount_quote = order.amount.checked_mul_dec_ceil(price)?;

            (price, amount_quote, Coin {
                denom: order.base_denom.clone(),
                amount: *order.amount,
            })
        },
    };

    ensure!(
        amount_quote >= pair.min_order_size,
        "order size ({} {}) is less than the minimum ({} {})",
        amount_quote,
        order.quote_denom,
        pair.min_order_size,
        order.quote_denom
    );

    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    // For BUY orders, invert the order ID. This is necessary for enforcing
    // price-time priority. See the docs on `OrderId` for details.
    if order.direction == Direction::Bid {
        order_id = !order_id;
    }

    events.push(OrderCreated {
        user,
        id: order_id,
        kind: OrderKind::Market,
        base_denom: order.base_denom.clone(),
        quote_denom: order.quote_denom.clone(),
        direction: order.direction,
        price: None,
        amount: *order.amount,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    MARKET_ORDERS.save(
        storage,
        (user, order_id),
        &(
            (
                (order.base_denom, order.quote_denom),
                order.direction,
                price,
                order_id,
            ),
            Order {
                user,
                id: order_id,
                kind: OrderKind::Market,
                price,
                amount: *order.amount,
                remaining: order.amount.checked_into_dec()?,
                created_at_block_height: Some(current_block_height),
            },
        ),
    )?;

    // Note: no need to change depth for market orders, since market orders are
    // canceled at the end of the block.

    Ok(())
}
