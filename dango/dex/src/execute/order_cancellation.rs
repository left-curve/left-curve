use {
    crate::{ORDERS, OrderKey, PAIRS, liquidity_depth::decrease_liquidity_depths},
    anyhow::ensure,
    dango_types::dex::{Direction, Order, OrderCanceled, OrderId, TimeInForce},
    grug::{
        Addr, DecCoin, DecCoins, EventBuilder, Number, Order as IterationOrder, StdResult, Storage,
        TransferBuilder,
    },
};

/// Cancel all orders from all users.
pub(super) fn cancel_all_orders(
    storage: &mut dyn Storage,
) -> anyhow::Result<(EventBuilder, TransferBuilder<DecCoins<6>>)> {
    let mut events = EventBuilder::new();
    let mut refunds = TransferBuilder::<DecCoins<6>>::new();

    for (order_key, order) in ORDERS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(
            storage,
            order_key.clone(),
            order,
            &mut events,
            refunds.get_mut(order.user),
        )?;

        ORDERS.remove(storage, order_key)?;
    }

    Ok((events, refunds))
}

/// Cancel all orders that belong to the given user.
pub(super) fn cancel_all_orders_from_user(
    storage: &mut dyn Storage,
    user: Addr,
    events: &mut EventBuilder,
    refunds: &mut DecCoins<6>,
) -> anyhow::Result<()> {
    // Cancel maker orders, meaning limit orders that are already in the book.
    for (order_key, order) in ORDERS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(storage, order_key.clone(), order, events, refunds)?;

        ORDERS.remove(storage, order_key)?;
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
    refunds: &mut DecCoins<6>,
) -> anyhow::Result<()> {
    let (order_key, order) = ORDERS.idx.order_id.load(storage, order_id)?;

    ensure!(
        order.user == user,
        "limit order `{order_id}` does not belong to the sender",
    );

    cancel_order(storage, order_key.clone(), order, events, refunds)?;

    ORDERS.remove(storage, order_key)?;

    Ok(())
}

fn cancel_order(
    storage: &mut dyn Storage,
    order_key: OrderKey,
    order: Order,
    events: &mut EventBuilder,
    refunds: &mut DecCoins<6>,
) -> anyhow::Result<()> {
    let ((base_denom, quote_denom), direction, price, order_id) = order_key;
    let remaining_in_quote = order.remaining.checked_mul(price)?;

    // If the order is GTC, decrease the liquidity depth.
    if order.time_in_force == TimeInForce::GoodTilCanceled {
        let pair = PAIRS.load(storage, (&base_denom, &quote_denom))?;

        decrease_liquidity_depths(
            storage,
            &base_denom,
            &quote_denom,
            direction,
            price,
            order.remaining,
            &pair.bucket_sizes,
        )?;
    }

    // Compute the amount of tokens to be sent back to the user.
    let refund = match direction {
        Direction::Bid => DecCoin {
            denom: quote_denom.clone(),
            amount: remaining_in_quote,
        },
        Direction::Ask => DecCoin {
            denom: base_denom.clone(),
            amount: order.remaining,
        },
    };

    events.push(OrderCanceled {
        user: order.user,
        id: order_id,
        time_in_force: TimeInForce::GoodTilCanceled,
        remaining: order.remaining,
        refund: refund.clone(),
        base_denom,
        quote_denom,
        direction,
        price,
        amount: order.amount,
    })?;

    refunds.insert(refund)?;

    Ok(())
}
