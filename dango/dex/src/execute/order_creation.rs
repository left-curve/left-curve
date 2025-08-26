use {
    crate::{LIMIT_ORDERS, MARKET_ORDERS, NEXT_ORDER_ID, PAIRS, RESTING_ORDER_BOOK},
    anyhow::{anyhow, ensure},
    dango_types::dex::{
        CreateLimitOrderRequest, CreateMarketOrderRequest, Direction, LimitOrder, MarketOrder,
        OrderCreated, OrderKind,
    },
    grug::{
        Addr, Coin, Coins, EventBuilder, MultiplyFraction, Number, NumberConst, Storage, Udec128_24,
    },
};

pub(super) fn create_limit_order(
    storage: &mut dyn Storage,
    current_block_height: u64,
    user: Addr,
    order: CreateLimitOrderRequest,
    events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
    let (base_denom, quote_denom, price, amount_base, deposit, direction) = match order {
        CreateLimitOrderRequest::Bid {
            base_denom,
            quote_denom,
            amount_quote,
            price,
        } => {
            let amount_base = amount_quote.checked_div_dec_floor(*price)?;
            let deposit = Coin {
                denom: quote_denom.clone(),
                amount: *amount_quote,
            };
            (
                base_denom,
                quote_denom,
                price,
                amount_base,
                deposit,
                Direction::Bid,
            )
        },
        CreateLimitOrderRequest::Ask {
            base_denom,
            quote_denom,
            amount_base,
            price,
        } => {
            let deposit = Coin {
                denom: base_denom.clone(),
                amount: *amount_base,
            };
            (
                base_denom,
                quote_denom,
                price,
                *amount_base,
                deposit,
                Direction::Ask,
            )
        },
    };

    ensure!(
        PAIRS.has(storage, (&base_denom, &quote_denom)),
        "pair not found with base `{}` and quote `{}`",
        base_denom,
        quote_denom
    );

    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    // For BUY orders, invert the order ID. This is necessary for enforcing
    // price-time priority. See the docs on `OrderId` for details.
    if direction == Direction::Bid {
        order_id = !order_id;
    }

    events.push(OrderCreated {
        user,
        id: order_id,
        kind: OrderKind::Limit,
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        direction,
        price: Some(*price),
        amount: amount_base,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    LIMIT_ORDERS.save(
        storage,
        ((base_denom, quote_denom), direction, *price, order_id),
        &LimitOrder {
            user,
            id: order_id,
            price: *price,
            amount: amount_base,
            remaining: amount_base.checked_into_dec()?,
            created_at_block_height: current_block_height,
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
    let (base_denom, quote_denom) = match &order {
        CreateMarketOrderRequest::Bid {
            base_denom,
            quote_denom,
            ..
        }
        | CreateMarketOrderRequest::Ask {
            base_denom,
            quote_denom,
            ..
        } => (base_denom, quote_denom),
    };

    ensure!(
        PAIRS.has(storage, (base_denom, quote_denom)),
        "pair not found with base `{}` and quote `{}`",
        base_denom,
        quote_denom
    );

    // Load the resting order book of the pair.
    // The best price available in the book, together with the order's maximum
    // slippage, will be used to determine the order's "limit price"
    let resting_order_book = RESTING_ORDER_BOOK
        .load(storage, (base_denom, quote_denom))
        .map_err(|err| {
            anyhow!(
                "can't create market order, because resting order book either doesn't exist or is corrupted. base denom: {}, quote denom: {}, err: {err}",
                base_denom,
                quote_denom
            )
        })?;

    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    let (price, direction, deposit, amount_base) = match order {
        CreateMarketOrderRequest::Bid {
            amount_quote,
            max_slippage,
            ..
        } => {
            let best_ask_price = resting_order_book.best_ask_price.ok_or_else(|| {
                anyhow!(
                    "can't create market bid order, because best ask price isn't available. base denom: {}, quote denom: {}",
                    base_denom,
                    quote_denom
                )
            })?;

            let one_add_max_slippage = Udec128_24::ONE.saturating_add(*max_slippage);
            let price = best_ask_price.saturating_mul(one_add_max_slippage);

            // For BUY orders, invert the order ID. This is necessary for enforcing
            // price-time priority. See the docs on `OrderId` for details.
            order_id = !order_id;

            (
                price,
                Direction::Bid,
                Coin {
                    denom: quote_denom.clone(),
                    amount: *amount_quote,
                },
                (*amount_quote).checked_div_dec_floor(price)?,
            )
        },
        CreateMarketOrderRequest::Ask {
            amount_base,
            max_slippage,
            ..
        } => {
            let best_bid_price = resting_order_book.best_bid_price.ok_or_else(|| {
                anyhow!(
                    "can't create market ask order, because best bid price isn't available. base denom: {}, quote denom: {}",
                    base_denom,
                    quote_denom
                )
            })?;

            let one_sub_max_slippage = Udec128_24::ONE.saturating_sub(*max_slippage);
            let price = best_bid_price.saturating_mul(one_sub_max_slippage);

            (
                price,
                Direction::Ask,
                Coin {
                    denom: base_denom.clone(),
                    amount: *amount_base,
                },
                *amount_base,
            )
        },
    };

    events.push(OrderCreated {
        user,
        id: order_id,
        kind: OrderKind::Market,
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        direction,
        price: None,
        amount: amount_base,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    MARKET_ORDERS.save(
        storage,
        (user, order_id),
        &(
            (
                (base_denom.clone(), quote_denom.clone()),
                direction,
                price,
                order_id,
            ),
            MarketOrder {
                user,
                id: order_id,
                price,
                amount: amount_base,
                remaining: amount_base.checked_into_dec()?,
                created_at_block_height: current_block_height,
            },
        ),
    )?;

    Ok(())
}
