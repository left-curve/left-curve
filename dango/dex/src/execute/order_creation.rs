use {
    crate::{
        NEXT_ORDER_ID, ORDERS, PAIRS, RESTING_ORDER_BOOK,
        liquidity_depth::increase_liquidity_depths,
    },
    anyhow::{anyhow, ensure},
    dango_types::dex::{
        AmountOption, CreateOrderRequest, Direction, Order, OrderCreated, PriceOption, TimeInForce,
    },
    grug::{Addr, Coins, EventBuilder, MultiplyFraction, Number, NumberConst, Storage, Udec128_24},
};

pub(super) fn create_order(
    storage: &mut dyn Storage,
    current_block_height: u64,
    user: Addr,
    order: CreateOrderRequest,
    events: &mut EventBuilder,
    deposits: &mut Coins,
) -> anyhow::Result<()> {
    // Load pair parameters.
    let pair = PAIRS
        .may_load(storage, (&order.base_denom, &order.quote_denom))?
        .ok_or_else(|| {
            anyhow!(
                "pair not found with base `{}` and quote `{}`",
                order.base_denom,
                order.quote_denom
            )
        })?;

    let direction = order.direction();
    let deposit = order.deposit();

    // Determine the order's price.
    let price = match order.price {
        PriceOption::BestAvailable { max_slippage } => match direction {
            Direction::Bid => {
                let best_ask_price = RESTING_ORDER_BOOK
                    .may_load(storage, (&order.base_denom, &order.quote_denom))?
                    .and_then(|book| book.best_ask_price)
                    .ok_or_else(|| {
                        anyhow!(
                            "best ask price isn't available for base: {}, quote: {}",
                            order.base_denom,
                            order.quote_denom
                        )
                    })?;
                let one_add_max_slippage = Udec128_24::ONE.saturating_add(*max_slippage);
                best_ask_price.saturating_mul(one_add_max_slippage)
            },
            Direction::Ask => {
                let best_bid_price = RESTING_ORDER_BOOK
                    .may_load(storage, (&order.base_denom, &order.quote_denom))?
                    .and_then(|book| book.best_bid_price)
                    .ok_or_else(|| {
                        anyhow!(
                            "best bid price isn't available for base: {}, quote: {}",
                            order.base_denom,
                            order.quote_denom
                        )
                    })?;
                let one_sub_max_slippage = Udec128_24::ONE.saturating_sub(*max_slippage);
                best_bid_price.saturating_mul(one_sub_max_slippage)
            },
        },
        PriceOption::Fixed(price) => *price,
    };

    // Determine the order's size.
    let (amount, amount_in_quote) = match order.amount {
        AmountOption::Bid { quote } => {
            let amount = quote.checked_div_dec_floor(price)?;
            (amount, *quote)
        },
        AmountOption::Ask { base } => {
            let amount_in_quote = base.checked_mul_dec_floor(price)?;
            (*base, amount_in_quote)
        },
    };

    let remaining = amount.checked_into_dec()?;

    // Ensure the order's size isn't too small.
    ensure!(
        amount_in_quote >= pair.min_order_size,
        "order size ({} {}) is less than the minimum ({} {})",
        amount_in_quote,
        order.quote_denom,
        pair.min_order_size,
        order.quote_denom
    );

    // Generate the order's ID.
    // See the docs on `OrderId` on why we need to bitwise invert it for BUY orders.
    let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;
    if direction == Direction::Bid {
        order_id = !order_id;
    }

    // Update contract storage:
    // - save the order in the `ORDERS` map;
    // - if the order is GTC, increase liquidity depths;
    ORDERS.save(
        storage,
        (
            (order.base_denom.clone(), order.quote_denom.clone()),
            direction,
            price,
            order_id,
        ),
        &Order {
            user,
            id: order_id,
            direction,
            time_in_force: order.time_in_force,
            price,
            amount,
            remaining,
            created_at_block_height: Some(current_block_height),
        },
    )?;

    if order.time_in_force == TimeInForce::GoodTilCanceled {
        increase_liquidity_depths(
            storage,
            &order.base_denom,
            &order.quote_denom,
            direction,
            price,
            remaining,
            &pair.bucket_sizes,
        )?;
    }

    // Emit event and increase expected deposit.
    events.push(OrderCreated {
        user,
        id: order_id,
        time_in_force: order.time_in_force,
        base_denom: order.base_denom,
        quote_denom: order.quote_denom,
        direction,
        price,
        amount,
        deposit: deposit.clone(),
    })?;

    deposits.insert(deposit)?;

    Ok(())
}
