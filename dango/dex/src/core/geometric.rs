use {
    crate::{MarketOrder, MergedOrders, core::market_order},
    anyhow::anyhow,
    dango_oracle::OracleQuerier,
    dango_types::dex::Direction,
    grug::{
        Addr, Bounded, Coin, CoinPair, Denom, IsZero, MultiplyFraction, Number, NumberConst,
        Order as IterationOrder, StdResult, Udec128, Uint128, ZeroExclusiveOneExclusive,
        ZeroExclusiveOneInclusive,
    },
    std::iter,
};

const GEOMETRIC_POOL_INITIAL_MINT_MULTIPLIER: Uint128 = Uint128::new(1000000);

pub fn add_initial_liquidity(
    oracle_querier: &mut OracleQuerier,
    deposit: &CoinPair,
) -> anyhow::Result<Uint128> {
    let deposit_value = oracle_value(oracle_querier, deposit)?;

    Ok(GEOMETRIC_POOL_INITIAL_MINT_MULTIPLIER.checked_mul_dec_floor(deposit_value)?)
}

pub fn add_subsequent_liquidity(
    oracle_querier: &mut OracleQuerier,
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec128> {
    let deposit_value = oracle_value(oracle_querier, &deposit)?;
    let reserve_value = oracle_value(oracle_querier, reserve)?;

    reserve.merge(deposit)?;

    Ok(deposit_value.checked_div(reserve_value.checked_add(deposit_value)?)?)
}

pub fn swap_exact_amount_in(
    oracle_querier: &mut OracleQuerier,
    base_denom: Denom,
    quote_denom: Denom,
    reserve: &CoinPair,
    input: &Coin,
    ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    order_spacing: Udec128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<Uint128> {
    let market_order_direction = if base_denom == input.denom {
        Direction::Ask
    } else if quote_denom == input.denom {
        Direction::Bid
    } else {
        unreachable!(
            "input denom (`{}`) is neither the base denom (`{}`) nor the quote denom (`{}`)",
            input.denom, base_denom, quote_denom
        );
    };

    // Reflect the curve
    let (passive_bids, passive_asks) = reflect_curve(
        oracle_querier,
        base_denom,
        quote_denom,
        reserve,
        ratio,
        order_spacing,
        swap_fee_rate,
    )?;

    // Construct the passive orders iterator. Asks for a bid, and bids for an ask.
    let mut passive_orders = match market_order_direction {
        Direction::Bid => MergedOrders::new(
            Box::new(iter::empty()),
            passive_asks,
            IterationOrder::Ascending,
            Addr::mock(0),
        )
        .peekable(),
        Direction::Ask => MergedOrders::new(
            Box::new(iter::empty()),
            passive_bids,
            IterationOrder::Descending,
            Addr::mock(0),
        )
        .peekable(),
    };

    // Construct the market order iterator.
    let mut market_orders = vec![(1u64, MarketOrder {
        // Won't be used. Only used when processing the filling outcomes in
        // `cron_execute` which will not be called here.
        user: Addr::mock(0),
        amount: input.amount,
        // Slippage control is implemented in the top level `swap_exact_amount_in`
        // function. We allow maximum slippage here to simply get an out amount.
        // The swap will be failed on top level if the slippage is too high.
        max_slippage: Udec128::new_permille(999),
    })]
    .into_iter()
    .peekable();

    // Match and fill the market order with the passive orders.
    let filling_outcomes = market_order::match_and_fill_market_orders(
        &mut market_orders,
        &mut passive_orders,
        market_order_direction,
        // Setting fee rates to zero to just get the result of the pure matching. Swap fee is applied later.
        Udec128::ZERO,
        Udec128::ZERO,
        // This parameter is used to determine whether a limit order is a maker
        // or a taker order. This isn't relevant here, as we're matching against
        // the passive pool orders, which don't pay maker/taker fees.
        0,
    )?;

    // Get the output amount from the filling outcomes.
    let output_amount = filling_outcomes
        .iter()
        .find(|outcome| outcome.order_id == 1u64)
        .map(|outcome| match market_order_direction {
            Direction::Bid => outcome.refund_base,
            Direction::Ask => outcome.refund_quote,
        })
        .ok_or(anyhow!(
            "failed to match market order in `swap_exact_amount_in`"
        ))?;

    Ok(output_amount.checked_mul_dec_floor(Udec128::ONE - *swap_fee_rate)?)
}

pub fn swap_exact_amount_out() -> StdResult<Uint128> {
    // FIXME
    todo!();
}

pub fn reflect_curve(
    oracle_querier: &mut OracleQuerier,
    base_denom: Denom,
    quote_denom: Denom,
    reserve: &CoinPair,
    ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    order_spacing: Udec128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<(
    Box<dyn Iterator<Item = (Udec128, Uint128)>>,
    Box<dyn Iterator<Item = (Udec128, Uint128)>>,
)> {
    // Compute the spot price. We will place orders above and below this price.
    let base_denom_price = oracle_querier
        .query_price(&base_denom, None)?
        .value_of_unit_amount(*reserve.first().amount)?;
    let quote_denom_price = oracle_querier
        .query_price(&quote_denom, None)?
        .value_of_unit_amount(*reserve.second().amount)?;

    let spot_price = base_denom_price.checked_div(quote_denom_price)?;

    let base_reserve = reserve.amount_of(&base_denom)?;
    let quote_reserve = reserve.amount_of(&quote_denom)?;

    // Construct bid price iterator with decreasing prices.
    let bids = {
        let one_sub_fee_rate = Udec128::ONE.checked_sub(*swap_fee_rate)?;
        let bid_starting_price = spot_price.checked_mul(one_sub_fee_rate)?;
        let mut maybe_price = Some(bid_starting_price);
        let mut remaining_quote = quote_reserve;

        iter::from_fn(move || {
            let price = maybe_price?;
            if price.is_zero() {
                return None;
            }

            let size_in_quote = remaining_quote.checked_mul_dec(*ratio).ok()?;
            let size = size_in_quote.checked_div_dec_floor(price).ok()?;

            maybe_price = price.checked_sub(order_spacing).ok();
            remaining_quote.checked_sub_assign(size_in_quote).ok()?;

            Some((price, size))
        })
    };

    // Construct ask price iterator with increasing prices.
    let asks = {
        let one_plus_fee_rate = Udec128::ONE.checked_add(*swap_fee_rate)?;
        let ask_starting_price = spot_price.checked_mul(one_plus_fee_rate)?;
        let mut maybe_price = Some(ask_starting_price);
        let mut remaining_base = base_reserve;

        iter::from_fn(move || {
            let price = maybe_price?;
            let size = remaining_base.checked_mul_dec(*ratio).ok()?;

            maybe_price = price.checked_add(order_spacing).ok();
            remaining_base.checked_sub_assign(size).ok()?;

            Some((price, size))
        })
    };

    Ok((Box::new(bids), Box::new(asks)))
}

fn oracle_value(
    oracle_querier: &mut OracleQuerier,
    coin_pair: &CoinPair,
) -> anyhow::Result<Udec128> {
    let first = coin_pair.first();
    let first_price = oracle_querier.query_price(first.denom, None)?;
    let first_value = first_price.value_of_unit_amount(*first.amount)?;

    let second = coin_pair.second();
    let second_price = oracle_querier.query_price(second.denom, None)?;
    let second_value = second_price.value_of_unit_amount(*second.amount)?;

    Ok(first_value.checked_add(second_value)?)
}
