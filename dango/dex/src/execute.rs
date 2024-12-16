use {
    crate::{NEXT_ORDER_ID, NEXT_PAIR_ID, ORDERS, PAIRS},
    anyhow::{bail, ensure},
    dango_types::dex::{ExecuteMsg, InstantiateMsg, Order, OrderId, OrderSide, Pair, PairId},
    grug::{
        btree_map, Coins, Message, MutableCtx, NumberConst, Response, StdResult, SudoCtx, Udec128,
        Uint128,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::CreatePair(pair) => create_pair(ctx, pair),
        ExecuteMsg::SubmitOrder {
            pair_id,
            limit_price,
        } => submit_order(ctx, pair_id, limit_price),
        ExecuteMsg::CancelOrder { order_id } => cancel_order(ctx, order_id),
    }
}

fn create_pair(ctx: MutableCtx, pair: Pair) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    ensure!(
        ctx.sender == cfg.owner,
        "only chain owner can create trading pairs"
    );

    // TODO: add a `contains` method to `UniqueIndex`
    ensure!(
        PAIRS
            .idx
            .denoms
            .may_load(
                ctx.storage,
                (pair.base_denom.clone(), pair.quote_denom.clone())
            )?
            .is_none(),
        "pair with base denom {} and quote denom {} already exists",
        pair.base_denom,
        pair.quote_denom
    );

    ensure!(
        PAIRS
            .idx
            .denoms
            .may_load(
                ctx.storage,
                (pair.quote_denom.clone(), pair.base_denom.clone())
            )?
            .is_none(),
        "pair with quote denom {} and base denom {} already exists",
        pair.base_denom,
        pair.quote_denom
    );

    let (pair_id, _) = NEXT_PAIR_ID.increment(ctx.storage)?;

    PAIRS.save(ctx.storage, pair_id, &pair)?;

    Ok(Response::new())
}

fn submit_order(
    ctx: MutableCtx,
    pair_id: PairId,
    limit_price: Option<Udec128>,
) -> anyhow::Result<Response> {
    let offer = ctx.funds.into_one_coin()?;
    let pair = PAIRS.load(ctx.storage, pair_id)?;

    let side = if offer.denom == pair.quote_denom {
        OrderSide::Buy
    } else if offer.denom == pair.base_denom {
        OrderSide::Sell
    } else {
        bail!(
            "invalid offer denom: {}! must be either quote denom {} or base denom {}",
            offer.denom,
            pair.quote_denom,
            pair.base_denom
        );
    };

    let limit_price = limit_price.unwrap_or_else(|| {
        if side == OrderSide::Buy {
            Udec128::MAX
        } else {
            Udec128::MIN
        }
    });

    let (order_id, _) = NEXT_ORDER_ID.increment(ctx.storage)?;

    ORDERS.save(ctx.storage, (pair_id, side, limit_price), &Order {
        order_id,
        maker: ctx.sender,
        size: offer.amount,
        filled: Uint128::ZERO,
    })?;

    Ok(Response::new())
}

fn cancel_order(ctx: MutableCtx, order_id: OrderId) -> anyhow::Result<Response> {
    let ((pair_id, side, _), order) = ORDERS.idx.order_id.load(ctx.storage, order_id)?;

    ensure!(
        order.maker == ctx.sender,
        "only the maker can cancel the order"
    );

    debug_assert!(
        order.size > order.filled,
        "filled amount is not less than size for an active order! size: {}, filled: {}",
        order.size,
        order.filled
    );

    let pair = PAIRS.load(ctx.storage, pair_id)?;

    let refund_denom = if side == OrderSide::Buy {
        pair.quote_denom
    } else {
        pair.base_denom
    };

    let refund_amount = order.size - order.filled;

    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        Coins::new_unchecked(btree_map! { refund_denom => refund_amount }),
    )?))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(_ctx: SudoCtx) -> anyhow::Result<Response> {
    todo!()
}
