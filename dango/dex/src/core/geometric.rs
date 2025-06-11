use {
    crate::core::xyk,
    dango_oracle::OracleQuerier,
    grug::{
        Bounded, CoinPair, IsZero, MathResult, MultiplyFraction, Number, NumberConst, StdResult,
        Udec128, Uint128, ZeroExclusiveOneExclusive, ZeroExclusiveOneInclusive,
    },
    std::iter,
};

pub fn add_initial_liquidity(deposit: &CoinPair) -> MathResult<Uint128> {
    // FIXME: Use oracle price to compute amoutn of intial LP tokens.
    xyk::normalized_invariant(deposit)
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

pub fn swap_exact_amount_in() -> StdResult<Uint128> {
    // FIXME
    todo!();
}

pub fn swap_exact_amount_out() -> StdResult<Uint128> {
    // FIXME
    todo!();
}

pub fn reflect_curve(
    base_reserve: Uint128,
    quote_reserve: Uint128,
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
        let ask_starting_price = marginal_price.checked_mul(one_plus_fee_rate)?;
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
