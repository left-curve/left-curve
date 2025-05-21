use {
    crate::{INCOMING_ORDERS, LimitOrder, MARKET_ORDERS, MarketOrder, NEXT_ORDER_ID, PAIRS},
    anyhow::ensure,
    dango_types::dex::{
        CreateLimitOrderRequest, CreateMarketOrderRequest, Direction, OrderSubmitted,
    },
    grug::{Addr, Coin, Coins, EventBuilder, MultiplyFraction, Storage},
};

pub(super) fn create_limit_order(
    storage: &mut dyn Storage,
    current_block_height: u64,
    user: Addr,
    order: CreateLimitOrderRequest,
    events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
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
        user,
        base_denom: order.base_denom.clone(),
        quote_denom: order.quote_denom.clone(),
        direction: order.direction,
        price: order.price,
        amount: order.amount,
        deposit,
    })?;

    INCOMING_ORDERS.save(
        storage,
        (user, order_id),
        &(
            (
                (order.base_denom, order.quote_denom),
                order.direction,
                order.price,
                order_id,
            ),
            LimitOrder {
                user,
                amount: order.amount,
                remaining: order.amount,
                created_at_block_height: current_block_height,
            },
        ),
    )?;

    Ok(())
}

pub(super) fn create_market_order(
    storage: &mut dyn Storage,
    user: Addr,
    order: CreateMarketOrderRequest,
    _events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
    ensure!(
        PAIRS.has(storage, (&order.base_denom, &order.quote_denom)),
        "pair not found with base `{}` and quote `{}`",
        order.base_denom,
        order.quote_denom
    );

    let deposit = match order.direction {
        Direction::Bid => Coin {
            denom: order.quote_denom.clone(),
            amount: order.amount,
        },
        Direction::Ask => Coin {
            denom: order.base_denom.clone(),
            amount: order.amount,
        },
    };

    let (order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    // TODO: add event

    deposits.insert(deposit)?;

    MARKET_ORDERS.save(
        storage,
        (
            (order.base_denom, order.quote_denom),
            order.direction,
            order_id,
        ),
        &MarketOrder {
            user,
            amount: order.amount,
        },
    )?;

    Ok(())
}
