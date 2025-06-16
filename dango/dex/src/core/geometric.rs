use {
    dango_oracle::OracleQuerier,
    grug::{
        Bounded, Coin, CoinPair, Denom, IsZero, MultiplyFraction, Number, NumberConst, StdResult,
        Udec128, Uint128, ZeroExclusiveOneExclusive, ZeroExclusiveOneInclusive,
    },
    std::{cmp, iter},
};

/// When adding liquidity for the first time into an empty pool, we determine
/// how many LP tokens to mint based on the USD value of the deposit.
const INITIAL_LP_TOKENS_PER_USD: Uint128 = Uint128::new(1_000_000);

pub fn add_initial_liquidity(
    oracle_querier: &mut OracleQuerier,
    deposit: &CoinPair,
) -> anyhow::Result<Uint128> {
    let deposit_value = oracle_value(oracle_querier, deposit)?;

    Ok(INITIAL_LP_TOKENS_PER_USD.checked_mul_dec(deposit_value)?)
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
    base_denom: &Denom,
    quote_denom: &Denom,
    input: &Coin,
    reserve: &CoinPair,
    ratio: Bounded<Udec128, ZeroExclusiveOneInclusive>,
    order_spacing: Udec128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<Uint128> {
    let (passive_bids, passive_asks) = reflect_curve(
        reserve.amount_of(base_denom)?,
        reserve.amount_of(quote_denom)?,
        ratio,
        order_spacing,
        swap_fee_rate,
    )?;

    // Pretend the input is a market order. Match it against the opposite side
    // of the passive limit orders.
    let output_amount = if input.denom == *base_denom {
        ask_exact_amount_in(input.amount, passive_bids)?
    } else if input.denom == *quote_denom {
        bid_exact_amount_in(input.amount, passive_asks)?
    } else {
        unreachable!(
            "input denom (`{}`) is neither the base (`{}`) nor the quote (`{}`). this should have been caught earlier.",
            input.denom, base_denom, quote_denom
        );
    };

    // Apply swap fee. Round so that user takes the loss.
    Ok(output_amount.checked_mul_dec_floor(Udec128::ONE - *swap_fee_rate)?)
}

fn bid_exact_amount_in(
    bid_amount_in_quote: Uint128,
    passive_asks: Box<dyn Iterator<Item = (Udec128, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_bid_in_quote = bid_amount_in_quote;
    let mut output_amount = Uint128::ZERO;

    for (price, size) in passive_asks {
        let matched_amount = cmp::min(size, remaining_bid_in_quote.checked_div_dec_floor(price)?);
        output_amount.checked_add_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_ceil(price)?;
        remaining_bid_in_quote.checked_sub_assign(matched_amount_in_quote)?;

        if remaining_bid_in_quote.is_zero() {
            break;
        }
    }

    Ok(output_amount)
}

fn ask_exact_amount_in(
    ask_amount: Uint128,
    passive_bids: Box<dyn Iterator<Item = (Udec128, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_ask = ask_amount;
    let mut output_amount_in_quote = Uint128::ZERO;

    for (price, size) in passive_bids {
        let matched_amount = cmp::min(size, remaining_ask);
        remaining_ask.checked_sub_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_floor(price)?;
        output_amount_in_quote.checked_add_assign(matched_amount_in_quote)?;

        if remaining_ask.is_zero() {
            break;
        }
    }

    Ok(output_amount_in_quote)
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
