use {
    crate::{
        LIMIT_ORDERS, MARKET_ORDERS, OrderKey, core::PairQuerier,
        liquidity_depth::decrease_liquidity_depths,
    },
    anyhow::{bail, ensure},
    dango_types::dex::{Direction, Order, OrderCanceled, OrderId, OrderKind},
    grug::{
        Addr, DecCoin, DecCoins, EventBuilder, Number, Order as IterationOrder, StdResult, Storage,
        TransferBuilder,
    },
};

/// Cancel all orders from all users.
pub(super) fn cancel_all_orders(
    storage: &mut dyn Storage,
    pair_querier: &mut PairQuerier,
) -> anyhow::Result<(EventBuilder, TransferBuilder<DecCoins<6>>)> {
    let mut events = EventBuilder::new();
    let mut refunds = TransferBuilder::<DecCoins<6>>::new();

    // Cancel limit orders.
    for (order_key, order) in LIMIT_ORDERS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(
            storage,
            order_key.clone(),
            order,
            &mut events,
            refunds.get_mut(order.user),
            pair_querier,
        )?;

        LIMIT_ORDERS.remove(storage, order_key)?;
    }

    // Cancel market orders.
    for ((user, order_id), (order_key, order)) in MARKET_ORDERS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(
            storage,
            order_key,
            order,
            &mut events,
            refunds.get_mut(order.user),
            pair_querier,
        )?;

        MARKET_ORDERS.remove(storage, (user, order_id));
    }

    Ok((events, refunds))
}

/// Cancel all orders that belong to the given user.
pub(super) fn cancel_all_orders_from_user(
    storage: &mut dyn Storage,
    user: Addr,
    events: &mut EventBuilder,
    refunds: &mut DecCoins<6>,
    pair_querier: &mut PairQuerier,
) -> anyhow::Result<()> {
    // Cancel maker orders, meaning limit orders that are already in the book.
    for (order_key, order) in LIMIT_ORDERS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(
            storage,
            order_key.clone(),
            order,
            events,
            refunds,
            pair_querier,
        )?;

        LIMIT_ORDERS.remove(storage, order_key)?;
    }

    // Cancel market orders.
    for ((pair, direction, price, order_id), order) in MARKET_ORDERS
        .prefix(user)
        .values(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        cancel_order(
            storage,
            (pair, direction, price, order_id),
            order,
            events,
            refunds,
            pair_querier,
        )?;

        MARKET_ORDERS.remove(storage, (user, order_id));
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
    pair_querier: &mut PairQuerier,
) -> anyhow::Result<()> {
    // We don't know whether the order is a limit or a market order.
    // First we check whether it's a limit order, which is the highest probability
    // situation.
    if let Some((order_key, order)) = LIMIT_ORDERS.idx.order_id.may_load(storage, order_id)? {
        ensure!(
            order.user == user,
            "limit order `{order_id}` does not belong to the sender",
        );

        cancel_order(
            storage,
            order_key.clone(),
            order,
            events,
            refunds,
            pair_querier,
        )?;

        LIMIT_ORDERS.remove(storage, order_key)?;

        return Ok(());
    }

    // Next, check whether it's a market order.
    if let Some((order_key, order)) = MARKET_ORDERS.may_load(storage, (user, order_id))? {
        ensure!(
            order.user == user,
            "market order `{order_id}` does not belong to the sender"
        );

        cancel_order(storage, order_key, order, events, refunds, pair_querier)?;

        MARKET_ORDERS.remove(storage, (user, order_id));

        return Ok(());
    }

    bail!("order not found with ID `{order_id}`");
}

fn cancel_order(
    storage: &mut dyn Storage,
    order_key: OrderKey,
    order: Order,
    events: &mut EventBuilder,
    refunds: &mut DecCoins<6>,
    pair_querier: &mut PairQuerier,
) -> anyhow::Result<()> {
    let ((base_denom, quote_denom), direction, price, order_id) = order_key;
    let remaining_in_quote = order.remaining.checked_mul(price)?;

    // If the order is a limit order, decrease the liquidity depth.
    if order.kind == OrderKind::Limit {
        let pair = pair_querier.query_pair(base_denom.clone(), quote_denom.clone())?;

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
            denom: quote_denom,
            amount: remaining_in_quote,
        },
        Direction::Ask => DecCoin {
            denom: base_denom,
            amount: order.remaining,
        },
    };

    events.push(OrderCanceled {
        user: order.user,
        id: order_id,
        kind: OrderKind::Limit,
        remaining: order.remaining,
        refund: refund.clone(),
    })?;

    refunds.insert(refund)?;

    Ok(())
}
