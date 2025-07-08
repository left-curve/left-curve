use {
    crate::{
        INCOMING_ORDERS, LIMIT_ORDERS, LimitOrder, LimitOrderKey, MARKET_ORDERS, MarketOrder,
        MarketOrderKey,
    },
    anyhow::{bail, ensure},
    dango_types::dex::{Direction, OrderCanceled, OrderId, OrderKind},
    grug::{Addr, Coin, Coins, EventBuilder, Number, Order as IterationOrder, StdResult, Storage},
};

/// Cancel all orders that belong to the given user.
pub(super) fn cancel_all_orders_from_user(
    storage: &mut dyn Storage,
    user: Addr,
    events: &mut EventBuilder,
    refunds: &mut Coins,
) -> StdResult<()> {
    // Cancel maker orders, meaning limit orders that are already in the book.
    for (order_key, order) in LIMIT_ORDERS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_limit_order(user, order_key.clone(), order, events, refunds)?;

        LIMIT_ORDERS.remove(storage, order_key)?;
    }

    // Cancel incoming limit orders.
    for ((pair, direction, price, order_id), order) in INCOMING_ORDERS
        .prefix(user)
        .values(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_limit_order(
            user,
            (pair, direction, price, order_id),
            order,
            events,
            refunds,
        )?;

        INCOMING_ORDERS.remove(storage, (user, order_id));
    }

    // Cancel market orders.
    for (order_key, order) in MARKET_ORDERS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_market_order(user, order_key.clone(), order, events, refunds)?;

        MARKET_ORDERS.remove(storage, order_key)?;
    }

    Ok(())
}

/// Cancel a single order by order ID, from the given user.
///
/// Error if the order doesn't belong to the user, or if the order doesn't exist.
pub(super) fn cancel_order_from_user(
    storage: &mut dyn Storage,
    user: Addr,
    order_id: OrderId,
    events: &mut EventBuilder,
    refunds: &mut Coins,
) -> anyhow::Result<()> {
    // We don't know whether the order is a maker order, an incoming limit order,
    // or a market order.
    // First we check whether it's a maker order, which is the highest probability
    // situation.
    if let Some((order_key, order)) = LIMIT_ORDERS.idx.order_id.may_load(storage, order_id)? {
        ensure!(
            order.user == user,
            "maker order `{order_id}` does not belong to the sender",
        );

        cancel_limit_order(user, order_key.clone(), order, events, refunds)?;

        LIMIT_ORDERS.remove(storage, order_key)?;

        return Ok(());
    }

    // Next, we check whether it's an incoming order.
    if let Some((order_key, order)) = INCOMING_ORDERS.may_load(storage, (user, order_id))? {
        ensure!(
            order.user == user,
            "incoming order `{order_id}` does not belong to the sender"
        );

        cancel_limit_order(user, order_key, order, events, refunds)?;

        INCOMING_ORDERS.remove(storage, (user, order_id));

        return Ok(());
    }

    // Finally, check whether it's a market order.
    if let Some((order_key, order)) = MARKET_ORDERS.idx.order_id.may_load(storage, order_id)? {
        ensure!(
            order.user == user,
            "market order `{order_id}` does not belong to the sender"
        );

        cancel_market_order(user, order_key.clone(), order, events, refunds)?;

        MARKET_ORDERS.remove(storage, order_key)?;

        return Ok(());
    }

    bail!("order not found with ID `{order_id}`");
}

fn cancel_limit_order(
    user: Addr,
    order_key: LimitOrderKey,
    order: LimitOrder,
    events: &mut EventBuilder,
    refunds: &mut Coins,
) -> StdResult<()> {
    let ((base_denom, quote_denom), direction, price, order_id) = order_key;

    // Compute the amount of tokens to be sent back to the user.
    let refund = match direction {
        Direction::Bid => Coin {
            denom: quote_denom,
            amount: order.remaining.checked_mul(price)?.into_int(),
        },
        Direction::Ask => Coin {
            denom: base_denom,
            amount: order.remaining.into_int(),
        },
    };

    events.push(OrderCanceled {
        user,
        id: order_id,
        kind: OrderKind::Limit,
        remaining: order.remaining.into_int(),
        refund: refund.clone(),
    })?;

    refunds.insert(refund)?;

    Ok(())
}

fn cancel_market_order(
    user: Addr,
    order_key: MarketOrderKey,
    order: MarketOrder,
    events: &mut EventBuilder,
    refunds: &mut Coins,
) -> StdResult<()> {
    let ((base_denom, quote_denom), direction, order_id) = order_key;

    // Compute the amount of tokens to be sent back to the user.
    let refund = match direction {
        Direction::Bid => Coin {
            denom: quote_denom,
            amount: order.amount,
        },
        Direction::Ask => Coin {
            denom: base_denom,
            amount: order.amount,
        },
    };

    events.push(OrderCanceled {
        user,
        id: order_id,
        kind: OrderKind::Market,
        remaining: order.amount,
        refund: refund.clone(),
    })?;

    refunds.insert(refund)?;

    Ok(())
}
