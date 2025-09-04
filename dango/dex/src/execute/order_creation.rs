use {
    crate::{
        LIMIT_ORDERS, MARKET_ORDERS, NEXT_ORDER_ID, PAIRS, RESTING_ORDER_BOOK,
        liquidity_depth::increase_liquidity_depths,
    },
    anyhow::{anyhow, ensure},
    dango_types::dex::{
        CreateLimitOrderRequest, CreateMarketOrderRequest, Direction, Order, OrderCreated,
        OrderKind,
    },
    grug::{
        Addr, Coin, Coins, EventBuilder, MultiplyFraction, NonZero, Number, NumberConst, Storage,
        Udec128_24, Uint128,
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
    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

    let (base_denom, quote_denom, price, amount_base, amount_quote, deposit, direction) =
        match order {
            CreateLimitOrderRequest::Bid {
                base_denom,
                quote_denom,
                amount_quote,
                price,
            } => {
                let ComputedBidAmounts {
                    amount_base,
                    amount_quote,
                } = compute_bid_amounts(*amount_quote, *price)?;

                // For BUY orders, invert the order ID. This is necessary for enforcing
                // price-time priority. See the docs on `OrderId` for details.
                order_id = !order_id;

                let deposit = Coin {
                    denom: quote_denom.clone(),
                    amount: amount_quote,
                };
                (
                    base_denom,
                    quote_denom,
                    *price,
                    amount_base,
                    amount_quote,
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
                let amount_quote = amount_base.checked_mul_dec_ceil(*price)?;
                (
                    base_denom,
                    quote_denom,
                    *price,
                    *amount_base,
                    amount_quote,
                    deposit,
                    Direction::Ask,
                )
            },
        };

    let pair = PAIRS
        .may_load(storage, (&base_denom, &quote_denom))?
        .ok_or_else(|| {
            anyhow!(
                "pair not found with base `{}` and quote `{}`",
                base_denom,
                quote_denom
            )
        })?;

    ensure!(
        amount_quote >= pair.min_order_size,
        "order size ({} {}) is less than the minimum ({} {})",
        amount_quote,
        quote_denom,
        pair.min_order_size,
        quote_denom
    );

    events.push(OrderCreated {
        user,
        id: order_id,
        kind: OrderKind::Limit,
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        direction,
        price: Some(price),
        amount: amount_base,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    let remaining = amount_base.checked_into_dec()?;

    increase_liquidity_depths(
        storage,
        &base_denom,
        &quote_denom,
        direction,
        price,
        remaining,
        &pair.bucket_sizes,
    )?;

    LIMIT_ORDERS.save(
        storage,
        ((base_denom, quote_denom), direction, price, order_id),
        &Order {
            user,
            id: order_id,
            kind: OrderKind::Limit,
            price,
            amount: amount_base,
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

    let pair = PAIRS
        .may_load(storage, (base_denom, quote_denom))?
        .ok_or_else(|| {
            anyhow!(
                "pair not found with base `{}` and quote `{}`",
                base_denom,
                quote_denom
            )
        })?;

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

    let (price, direction, deposit, amount_base, amount_quote) = match order {
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

            let ComputedBidAmounts {
                amount_base,
                amount_quote,
            } = compute_bid_amounts(*amount_quote, price)?;

            // For BUY orders, invert the order ID. This is necessary for enforcing
            // price-time priority. See the docs on `OrderId` for details.
            order_id = !order_id;

            (
                price,
                Direction::Bid,
                Coin {
                    denom: quote_denom.clone(),
                    amount: amount_quote,
                },
                amount_base,
                amount_quote,
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
            let amount_quote = amount_base.checked_mul_dec_ceil(price)?;

            (
                price,
                Direction::Ask,
                Coin {
                    denom: base_denom.clone(),
                    amount: *amount_base,
                },
                *amount_base,
                amount_quote,
            )
        },
    };

    ensure!(
        amount_quote >= pair.min_order_size,
        "order size ({} {}) is less than the minimum ({} {})",
        amount_quote,
        quote_denom,
        pair.min_order_size,
        quote_denom
    );

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
            Order {
                user,
                id: order_id,
                kind: OrderKind::Market,
                price,
                amount: amount_base,
                remaining: amount_base.checked_into_dec()?,
                created_at_block_height: Some(current_block_height),
            },
        ),
    )?;

    // Note: no need to change depth for market orders, since market orders are
    // canceled at the end of the block.

    Ok(())
}

fn compute_bid_amounts(
    amount_quote: Uint128,
    price: Udec128_24,
) -> anyhow::Result<ComputedBidAmounts> {
    let amount_base = NonZero::new(amount_quote.checked_div_dec_floor(price)?)?;
    // Is safe to use `checked_mul_dec_floor` instead of `checked_mul_dec_ceil`
    // because if the order is cancelled, we calculate the refund amount from the base
    // amount, which is always rounded down.
    // See proptests at dango/testing/tests/dex_proptests.rs:test_order_creation.
    let amount_quote = NonZero::new(amount_base.checked_mul_dec_floor(price)?)?;
    Ok(ComputedBidAmounts {
        amount_base: *amount_base,
        amount_quote: *amount_quote,
    })
}

struct ComputedBidAmounts {
    amount_base: Uint128,
    amount_quote: Uint128,
}
