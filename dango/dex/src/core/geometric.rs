use {
    anyhow::{bail, ensure},
    dango_oracle::OracleQuerier,
    dango_types::dex::{Geometric, Price},
    grug::{
        Bounded, Coin, CoinPair, Denom, IsZero, MultiplyFraction, Number, NumberConst, Udec128,
        Uint128, ZeroExclusiveOneExclusive,
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
) -> anyhow::Result<Price> {
    let deposit_value = oracle_value(oracle_querier, &deposit)?;
    let reserve_value = oracle_value(oracle_querier, reserve)?;

    reserve.merge(deposit)?;

    Ok(deposit_value.checked_div(reserve_value)?)
}

/// Note: this function does not concern the liquidity fee.
/// Liquidity fee logics are found in `PairParams::swap_exact_amount_in`, in `liquidity_pool.rs`.
pub fn swap_exact_amount_in(
    oracle_querier: &mut OracleQuerier,
    base_denom: &Denom,
    quote_denom: &Denom,
    input: &Coin,
    reserve: &CoinPair,
    params: Geometric,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<Uint128> {
    let (passive_bids, passive_asks) = reflect_curve(
        oracle_querier,
        base_denom,
        quote_denom,
        reserve.amount_of(base_denom)?,
        reserve.amount_of(quote_denom)?,
        params,
        swap_fee_rate,
    )?;

    // Pretend the input is a market order. Match it against the opposite side
    // of the passive limit orders.
    if input.denom == *base_denom {
        ask_exact_amount_in(input.amount, passive_bids)
    } else if input.denom == *quote_denom {
        bid_exact_amount_in(input.amount, passive_asks)
    } else {
        unreachable!(
            "input denom (`{}`) is neither the base (`{}`) nor the quote (`{}`). this should have been caught earlier.",
            input.denom, base_denom, quote_denom
        );
    }
}

// NOTE: Always round down (floor) the output amount; always round up (ceil) the input amount.
fn bid_exact_amount_in(
    bid_amount_in_quote: Uint128,
    passive_asks: Box<dyn Iterator<Item = (Price, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_bid_in_quote = bid_amount_in_quote.checked_into_dec::<6>()?;
    let mut output_amount = Udec128::ZERO;

    for (price, amount) in passive_asks {
        let remaining_bid = remaining_bid_in_quote.checked_div_dec_floor(price)?;
        let matched_amount = cmp::min(amount.checked_into_dec()?, remaining_bid);
        output_amount.checked_add_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_ceil(price)?;
        remaining_bid_in_quote.checked_sub_assign(matched_amount_in_quote)?;

        if remaining_bid_in_quote.into_int_floor().is_zero()
            || remaining_bid.is_zero()
            || matched_amount_in_quote.is_zero()
        {
            return Ok(output_amount.into_int());
        }
    }

    bail!("not enough liquidity to fulfill the swap! remaining amount: {remaining_bid_in_quote}");
}

fn ask_exact_amount_in(
    ask_amount: Uint128,
    passive_bids: Box<dyn Iterator<Item = (Price, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_ask = ask_amount.checked_into_dec::<6>()?;
    let mut output_amount_in_quote = Udec128::ZERO;

    for (price, amount) in passive_bids {
        let matched_amount = cmp::min(amount.checked_into_dec()?, remaining_ask);
        remaining_ask.checked_sub_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_floor(price)?;
        output_amount_in_quote.checked_add_assign(matched_amount_in_quote)?;

        if remaining_ask.is_zero() || matched_amount_in_quote.is_zero() {
            return Ok(output_amount_in_quote.into_int());
        }
    }

    bail!("not enough liquidity to fulfill the swap! remaining amount: {remaining_ask}")
}

/// Note: this function does not concern the liquidity fee.
/// Liquidity fee logics are found in `PairParams::swap_exact_amount_out`, in `liquidity_pool.rs`.
pub fn swap_exact_amount_out(
    oracle_querier: &mut OracleQuerier,
    base_denom: &Denom,
    quote_denom: &Denom,
    output: &Coin,
    reserve: &CoinPair,
    params: Geometric,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<Uint128> {
    let output_reserve = reserve.amount_of(&output.denom)?;
    ensure!(
        output_reserve > output.amount,
        "insufficient liquidity: {} <= {}",
        output_reserve,
        output.amount
    );

    let (passive_bids, passive_asks) = reflect_curve(
        oracle_querier,
        base_denom,
        quote_denom,
        reserve.amount_of(base_denom)?,
        reserve.amount_of(quote_denom)?,
        params,
        swap_fee_rate,
    )?;

    if output.denom == *base_denom {
        bid_exact_amount_out(output.amount, passive_asks)
    } else if output.denom == *quote_denom {
        ask_exact_amount_out(output.amount, passive_bids)
    } else {
        unreachable!(
            "output denom (`{}`) is neither the base (`{}`) nor the quote (`{}`). this should have been caught earlier.",
            output.denom, base_denom, quote_denom
        );
    }
}

fn bid_exact_amount_out(
    bid_amount: Uint128,
    passive_asks: Box<dyn Iterator<Item = (Price, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_bid = bid_amount.checked_into_dec::<6>()?;
    let mut input_amount = Udec128::ZERO;

    for (price, amount) in passive_asks {
        let matched_amount = cmp::min(amount.checked_into_dec()?, remaining_bid);
        remaining_bid.checked_sub_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_ceil(price)?;
        input_amount.checked_add_assign(matched_amount_in_quote)?;

        if remaining_bid.is_zero() || matched_amount_in_quote.is_zero() {
            return Ok(input_amount.into_int_ceil());
        }
    }

    bail!("not enough liquidity to fulfill the swap! remaining amount: {remaining_bid}")
}

fn ask_exact_amount_out(
    ask_amount_in_quote: Uint128,
    passive_bids: Box<dyn Iterator<Item = (Price, Uint128)>>,
) -> anyhow::Result<Uint128> {
    let mut remaining_ask_in_quote = ask_amount_in_quote.checked_into_dec::<6>()?;
    let mut input_amount = Udec128::ZERO;

    for (price, amount) in passive_bids {
        let remaining_ask = remaining_ask_in_quote.checked_div_dec_floor(price)?;
        let matched_amount = cmp::min(amount.checked_into_dec()?, remaining_ask);
        input_amount.checked_add_assign(matched_amount)?;

        let matched_amount_in_quote = matched_amount.checked_mul_dec_ceil(price)?;
        remaining_ask_in_quote.checked_sub_assign(matched_amount_in_quote)?;

        if remaining_ask_in_quote.into_int_floor().is_zero()
            || remaining_ask.is_zero()
            || matched_amount_in_quote.is_zero()
        {
            return Ok(input_amount.into_int_ceil());
        }
    }

    bail!("not enough liquidity to fulfill the swap! remaining amount: {remaining_ask_in_quote}")
}

pub fn reflect_curve(
    oracle_querier: &mut OracleQuerier,
    base_denom: &Denom,
    quote_denom: &Denom,
    base_reserve: Uint128,
    quote_reserve: Uint128,
    params: Geometric,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<(
    Box<dyn Iterator<Item = (Price, Uint128)>>,
    Box<dyn Iterator<Item = (Price, Uint128)>>,
)> {
    // Compute the price of the base asset denominated in the quote asset.
    // We will place orders above and below this price.
    //
    // Note that we aren't computing the price in the human units, but in their
    // base units. In other words, we don't want to know how many BTC is per USDC;
    // we want to know how many sat (1e-8 BTC) is per 1e-6 USDC.
    let marginal_price = {
        const PRECISION: Uint128 = Uint128::new(1_000_000);

        let base_price: Price = oracle_querier
            .query_price(base_denom, None)?
            .value_of_unit_amount(PRECISION)?;
        let quote_price: Price = oracle_querier
            .query_price(quote_denom, None)?
            .value_of_unit_amount(PRECISION)?;

        base_price.checked_div(quote_price)?
    };

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

            let size_in_quote = remaining_quote.checked_mul_dec(*params.ratio).ok()?;
            let size = size_in_quote.checked_div_dec_floor(price).ok()?;
            if size.is_zero() {
                return None;
            }

            // Fix: recompute the amount of quote asset used.
            // The order size is denominated in the base asset, so the actual
            // amount of quote asset used needs to be recomputed from the base amount.
            //
            // Fix 2025-10-15: here we should `ceil`, but previously we had `floor`.
            let size_in_quote = size.checked_mul_dec_ceil(price).ok()?;

            maybe_price = price.checked_sub(params.spacing).ok();
            remaining_quote.checked_sub_assign(size_in_quote).ok()?;

            Some((price, size))
        })
        .take(params.limit)
    };

    // Construct ask price iterator with increasing prices.
    let asks = {
        let one_plus_fee_rate = Udec128::ONE.checked_add(*swap_fee_rate)?;
        let ask_starting_price = marginal_price.checked_mul(one_plus_fee_rate)?;
        let mut maybe_price = Some(ask_starting_price);
        let mut remaining_base = base_reserve;

        iter::from_fn(move || {
            let price = maybe_price?;

            let size = remaining_base.checked_mul_dec(*params.ratio).ok()?;
            if size.is_zero() {
                return None;
            }

            maybe_price = price.checked_add(params.spacing).ok();
            remaining_base.checked_sub_assign(size).ok()?;

            Some((price, size))
        })
        .take(params.limit)
    };

    Ok((Box::new(bids), Box::new(asks)))
}

fn oracle_value(oracle_querier: &mut OracleQuerier, coin_pair: &CoinPair) -> anyhow::Result<Price> {
    let first = coin_pair.first();
    let first_price = oracle_querier.query_price(first.denom, None)?;
    let first_value = first_price.value_of_unit_amount(*first.amount)?;

    let second = coin_pair.second();
    let second_price = oracle_querier.query_price(second.denom, None)?;
    let second_value: Price = second_price.value_of_unit_amount(*second.amount)?;

    Ok(first_value.checked_add(second_value)?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            constants::{eth, usdc},
            oracle::PrecisionedPrice,
        },
        grug::{ResultExt, Timestamp, Udec128_24},
        std::str::FromStr,
    };

    #[test]
    fn test_ask_exact_amount_out_doesnt_fail_on_rounding_on_last_passive_order() {
        let passive_bids = vec![(Udec128_24::new(150), Uint128::new(100000000))];
        let ask_amount_in_quote = Uint128::new(100);

        ask_exact_amount_out(ask_amount_in_quote, Box::new(passive_bids.into_iter()))
            .should_succeed_and_equal(Uint128::ONE);
    }

    #[test]
    fn test_bid_exact_amount_in_doesnt_fail_on_rounding_on_last_passive_order() {
        let passive_asks = vec![(Udec128_24::new(150), Uint128::new(100000000))];
        let bid_amount_in_quote = Uint128::new(100);

        bid_exact_amount_in(bid_amount_in_quote, Box::new(passive_asks.into_iter()))
            .should_succeed_and_equal(Uint128::ZERO);
    }

    /// Ensure that the sum of the sizes of all the orders are less or equal than
    /// the pool's reserve.
    /// In order words, even if all the orders are filled at the worse possible
    /// prices (their limit prices), the pool must have enough reserve to cover
    /// the outflow.
    #[test]
    fn testnet_3_halt_20251015() {
        let eth_reserve = Uint128::new(491567617626054560353243);
        let usdc_reserve = Uint128::new(8);

        let (bids, asks) = reflect_curve(
            &mut OracleQuerier::new_mock(
                vec![
                    (
                        eth::DENOM.clone(),
                        PrecisionedPrice::new(
                            Udec128::from_str("4117.84677205").unwrap(),
                            Timestamp::from_millis(1760513220400),
                            18,
                        ),
                    ),
                    (
                        usdc::DENOM.clone(),
                        PrecisionedPrice::new(
                            Udec128::from_str("0.99996229").unwrap(),
                            Timestamp::from_millis(1760513220400),
                            6,
                        ),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
            &eth::DENOM,
            &usdc::DENOM,
            eth_reserve,
            usdc_reserve,
            Geometric {
                ratio: Bounded::new(Udec128::new_percent(60)).unwrap(),
                spacing: Udec128::from_str("0.000000000005").unwrap(),
                limit: 5,
            },
            Bounded::new(Udec128::from_str("0.00005").unwrap()).unwrap(),
        )
        .unwrap();

        // Sum the amount of all asks
        let asks_sum = asks.map(|(_, amount)| amount).sum::<Uint128>();
        assert!(asks_sum <= eth_reserve);

        // Sum the amount of all bids converted to quote asset
        let bids_sum = bids
            .map(|(price, amount)| amount.checked_mul_dec_ceil(price).unwrap())
            .sum::<Uint128>();
        assert!(bids_sum <= usdc_reserve);
    }
}
