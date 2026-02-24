use {
    crate::{ASKS, BIDS, USER_STATES},
    anyhow::{anyhow, ensure},
    dango_types::perps::OrderId,
    grug::{MutableCtx, Order as IterationOrder, Response, StdResult},
};

pub fn cancel_one_order(ctx: MutableCtx, order_id: OrderId) -> anyhow::Result<Response> {
    // Since we don't know whether it's a buy or a sell order, we first attempt
    // to load it from the `BIDS` map. If not found, load it from `ASKS`.
    // If still not found, bail.
    let (order_key, order) = BIDS
        .idx
        .order_id
        .may_load(ctx.storage, order_id)
        .transpose()
        .or_else(|| {
            ASKS.idx
                .order_id
                .may_load(ctx.storage, order_id)
                .transpose()
        })
        .ok_or_else(|| anyhow!("order not found with id {order_id}"))??;

    ensure!(
        ctx.sender == order.user,
        "you are not the owner of this order"
    );

    // Delete the order.
    if order.size.is_positive() {
        BIDS.remove(ctx.storage, order_key)?;
    } else {
        ASKS.remove(ctx.storage, order_key)?;
    }

    // Update user state: release reserved margin and decrement open order count.
    USER_STATES.modify(ctx.storage, ctx.sender, |mut user_state| -> StdResult<_> {
        user_state.open_order_count -= 1;
        (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;

        // Delete the user state if it's empty. Otherwise, save the updated user state.
        if user_state.is_empty() {
            Ok(None)
        } else {
            Ok(Some(user_state))
        }
    })?;

    Ok(Response::new())
}

pub fn cancel_all_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Load the sender's user state.
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // For bids and asks respectively, first collect all orders into memory,
    // then for each order, 1) delete, 2) release the reserved margin and decrement
    // open order count.
    for map in [BIDS, ASKS] {
        for (order_key, order) in map
            .idx
            .user
            .prefix(ctx.sender)
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?
        {
            map.remove(ctx.storage, order_key)?;

            user_state.open_order_count -= 1;
            (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;
        }
    }

    // Delete the user state if it's empty. Otherwise, save the updated user state.
    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender);
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    // TODO
}
