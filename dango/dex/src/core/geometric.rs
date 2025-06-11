use {
    crate::core::xyk,
    dango_oracle::OracleQuerier,
    grug::{
        Bounded, CoinPair, Inner, IsZero, MultiplyFraction, Number, NumberConst, StdResult,
        Udec128, Uint128, ZeroExclusiveOneExclusive, ZeroExclusiveOneInclusive,
    },
    std::iter,
};

// FIXME: Use oracle price to compute amoutn of intial LP tokens.
pub fn add_initial_liquidity(
    mut reserve: CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<(CoinPair, Uint128)> {
    reserve.merge(deposit)?;

    let mint_amount = xyk::normalized_invariant(&reserve)?;

    Ok((reserve, mint_amount))
}

pub fn add_subsequent_liquidity(
    oracle_querier: &mut OracleQuerier,
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec128> {
    let deposit_value = oracle_value(oracle_querier, &deposit)?;
    let reserve_value = oracle_value(oracle_querier, reserve)?;

    Ok(deposit_value.checked_div(reserve_value.checked_add(deposit_value)?)?)
}

pub fn swap_exact_amount_in() -> StdResult<Uint128> {
    // FIXME
    todo!();
}

pub fn swap_exact_amount_out() -> StdResult<Uint128> {
    // FIXME
    todo!();
}

pub fn reflect_curve(
    mut base_reserve: Uint128,
    mut quote_reserve: Uint128,
    ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    order_spacing: Udec128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> StdResult<(
    Box<dyn Iterator<Item = (Udec128, Uint128)>>,
    Box<dyn Iterator<Item = (Udec128, Uint128)>>,
)> {
    // FIXME: use oracle price instead
    let marginal_price = Udec128::checked_from_ratio(quote_reserve, base_reserve)?;

    // Construct bid price iterator with decreasing prices.
    let bids = {
        let one_sub_fee_rate = Udec128::ONE.checked_sub(*swap_fee_rate)?;
        let bid_starting_price = marginal_price.checked_mul(one_sub_fee_rate)?;
        let mut maybe_price = Some(bid_starting_price);

        let bid_prices = iter::from_fn(move || {
            let price = match maybe_price {
                Some(price) if price.is_non_zero() => price,
                _ => return None,
            };

            maybe_price = price.checked_sub(order_spacing).ok();

            Some(price)
        });

        let bid_sizes_in_quote = iter::from_fn(move || {
            let size = match quote_reserve.checked_mul_dec(ratio.into_inner()) {
                Ok(size_in_quote) => size_in_quote,
                Err(_) => return None,
            };

            quote_reserve.checked_sub_assign(size).ok()?;

            Some(size)
        });

        bid_prices
            .zip(bid_sizes_in_quote)
            .filter_map(|(price, size_in_quote)| {
                let size = size_in_quote.checked_div_dec_floor(price).ok()?;
                Some((price, size))
            })
    };

    // Construct ask price iterator with increasing prices.
    let asks = {
        let one_plus_fee_rate = Udec128::ONE.checked_add(*swap_fee_rate)?;
        let ask_starting_price = marginal_price.checked_mul(one_plus_fee_rate)?;
        let mut maybe_price = Some(ask_starting_price);

        let ask_prices = iter::from_fn(move || {
            let price = match maybe_price {
                Some(price) if price.is_non_zero() => price,
                _ => return None,
            };
            maybe_price = price.checked_add(order_spacing).ok();
            Some(price)
        });

        // Construct ask size placing `ratio` of the remaining liquidity of
        // base reserve into each order.
        let ask_sizes = iter::from_fn(move || {
            let size = match base_reserve.checked_mul_dec(ratio.into_inner()) {
                Ok(size) => size,
                Err(_) => return None,
            };
            base_reserve.checked_sub_assign(size).ok()?;
            Some(size)
        });

        ask_prices.zip(ask_sizes)
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
