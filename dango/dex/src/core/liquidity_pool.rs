use {
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::dex::{PairParams, PassiveLiquidity},
    grug::{
        Coin, CoinPair, Denom, Inner, IsZero, MathResult, MultiplyFraction, MultiplyRatio, Number,
        NumberConst, StdResult, Udec128, Uint128,
    },
    std::{cmp, iter},
};

const HALF: Udec128 = Udec128::new_percent(50);

pub trait PassiveLiquidityPool {
    /// Provide liquidity to the pool. This function mutates the pool reserves.
    /// Liquidity is provided at the current pool balance, any excess funds are
    /// returned to the user.
    ///
    /// ## Inputs
    ///
    /// - `oracle_querier`: The oracle querier.
    /// - `reserve`: The current pool reserves, before the deposit is added.
    /// - `lp_token_supply`: The current total supply of LP tokens.
    /// - `deposit`: The funds to add to the pool. Note, this may be asymmetrical,
    ///   or in the extreme case, one-sided.
    ///
    /// ## Outputs
    ///
    /// - The updated pool reserves.
    /// - The amount of LP tokens to mint.
    fn add_liquidity(
        &self,
        oracle_querier: &mut OracleQuerier,
        reserve: CoinPair,
        lp_token_supply: Uint128,
        deposit: CoinPair,
    ) -> anyhow::Result<(CoinPair, Uint128)>;

    /// Remove a portion of the liquidity from the pool. This function mutates the pool reserves.
    ///
    /// ## Inputs
    ///
    /// - `reserve`: The current pool reserves, before the withdrawal is made.
    /// - `lp_token_supply`: The current total supply of LP tokens.
    /// - `lp_burn_amount`: The amount of LP tokens to burn.
    ///
    /// ## Outputs
    ///
    /// - The updated pool reserves.
    /// - The funds withdrawn from the pool.
    fn remove_liquidity(
        &self,
        mut reserve: CoinPair,
        lp_token_supply: Uint128,
        lp_burn_amount: Uint128,
    ) -> anyhow::Result<(CoinPair, CoinPair)> {
        let refund = reserve.split(lp_burn_amount, lp_token_supply)?;

        Ok((reserve, refund))
    }

    /// Perform a swap with an exact amount of input and a variable output.
    ///
    /// ## Inputs
    ///
    /// - `reserve`: The current pool reserves, before the swap is performed.
    /// - `input`: The amount of input asset to swap.
    ///
    /// ## Outputs
    ///
    /// - The updated pool reserves.
    /// - The amount of output asset received from the swap.
    ///
    /// ## Notable errors
    ///
    /// The input asset must be one of the reserve assets, otherwise error.
    fn swap_exact_amount_in(
        &self,
        reserve: CoinPair,
        input: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)>;

    /// Perform a swap with a variable amount of input and an exact output.
    ///
    /// ## Inputs
    ///
    /// - `reserve`: The current pool reserves, before the swap is performed.
    /// - `output`: The amount of output asset to swap.
    ///
    /// ## Outputs
    ///
    /// - The updated pool reserves.
    /// - The necessary input asset.
    ///
    /// ## Notable errors
    ///
    /// The output asset must be one of the reserve assets, otherwise error.
    fn swap_exact_amount_out(
        &self,
        reserve: CoinPair,
        output: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)>;

    /// Reflect the curve onto the orderbook.
    ///
    /// ## Inputs
    ///
    /// - `base_denom`: The base asset of the pool.
    /// - `quote_denom`: The quote asset of the pool.
    /// - `reserve`: The current pool reserve.
    /// - `spread`: The spread between the ask and bid.
    ///
    /// ## Outputs
    ///
    /// - A tuple of two iterators of orders to place on the orderbook. The
    ///   first contains the bids, the second contains the asks, specified as a
    ///   tuple of (price, amount).
    ///
    /// ## Notes
    ///
    /// Note that the iterator item doesn't a `Result` type. If there is an
    /// error in computing the order, the iterator should return `None` and thus
    /// terminates.
    fn reflect_curve(
        self,
        base_denom: Denom,
        quote_denom: Denom,
        reserve: &CoinPair,
    ) -> StdResult<(
        Box<dyn Iterator<Item = (Udec128, Uint128)>>, // bids
        Box<dyn Iterator<Item = (Udec128, Uint128)>>, // asks
    )>;
}

impl PassiveLiquidityPool for PairParams {
    fn add_liquidity(
        &self,
        oracle_querier: &mut OracleQuerier,
        mut reserve: CoinPair,
        lp_token_supply: Uint128,
        deposit: CoinPair,
    ) -> anyhow::Result<(CoinPair, Uint128)> {
        let mint_ratio = match (&self.pool_type, lp_token_supply) {
            (PassiveLiquidity::Xyk { .. }, Uint128::ZERO) => {
                return xyk_add_initial_liquidity(reserve, deposit.clone());
            },
            (PassiveLiquidity::Geometric { .. }, Uint128::ZERO) => {
                return geometric_add_initial_liquidity(reserve, deposit.clone());
            },
            (PassiveLiquidity::Xyk { .. }, _) => {
                xyk_add_subsequent_liquidity(&mut reserve, deposit.clone())?
            },
            (PassiveLiquidity::Geometric { .. }, _) => {
                geometric_add_subsequent_liquidity(oracle_querier, &mut reserve, deposit.clone())?
            },
        };

        let mint_amount_before_fee = lp_token_supply.checked_mul_dec_floor(mint_ratio)?;

        // Apply swap fee to unbalanced provision. Logic is based on Curve V2:
        // https://github.com/curvefi/twocrypto-ng/blob/main/contracts/main/Twocrypto.vy#L1146-L1168
        let (a, b, reserve_a, reserve_b) = (
            *deposit.first().amount,
            *deposit.second().amount,
            *reserve.first().amount,
            *reserve.second().amount,
        );

        let sum_reserves = reserve_a.checked_add(reserve_b)?;
        let avg_reserves = sum_reserves.checked_div(Uint128::new(2))?;
        let fee_rate = Udec128::checked_from_ratio(
            abs_diff(a, avg_reserves).checked_add(abs_diff(b, avg_reserves))?,
            sum_reserves,
        )?
        .checked_mul(self.swap_fee_rate.checked_mul(HALF)?)?;

        let mint_amount =
            mint_amount_before_fee.checked_mul_dec_floor(Udec128::ONE.checked_sub(fee_rate)?)?;

        Ok((reserve, mint_amount))
    }

    fn swap_exact_amount_in(
        &self,
        mut reserve: CoinPair,
        input: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)> {
        let output_denom = if reserve.first().denom == &input.denom {
            reserve.second().denom.clone()
        } else {
            reserve.first().denom.clone()
        };

        let input_reserve = reserve.amount_of(&input.denom)?;
        let output_reserve = reserve.amount_of(&output_denom)?;

        let output_amount_after_fee = match self.pool_type {
            PassiveLiquidity::Xyk { .. } => {
                // Solve A * B = (A + input_amount) * (B - output_amount) for output_amount
                // => output_amount = B - (A * B) / (A + input_amount)
                // Round so that user takes the loss.
                let output_amount =
                    output_reserve.checked_sub(input_reserve.checked_multiply_ratio_ceil(
                        output_reserve,
                        input_reserve.checked_add(input.amount)?,
                    )?)?;

                // Apply swap fee. Round so that user takes the loss.
                output_amount
                    .checked_mul_dec_floor(Udec128::ONE - self.swap_fee_rate.into_inner())?
            },
            PassiveLiquidity::Geometric { .. } => {
                // TODO: implement
                todo!()
            },
        };

        let output = Coin {
            denom: output_denom,
            amount: output_amount_after_fee,
        };

        reserve.checked_add(&input)?.checked_sub(&output)?;

        Ok((reserve, output))
    }

    fn swap_exact_amount_out(
        &self,
        mut reserve: CoinPair,
        output: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)> {
        let input_denom = if reserve.first().denom == &output.denom {
            reserve.second().denom.clone()
        } else {
            reserve.first().denom.clone()
        };

        let input_reserve = reserve.amount_of(&input_denom)?;
        let output_reserve = reserve.amount_of(&output.denom)?;

        ensure!(
            output_reserve > output.amount,
            "insufficient liquidity: {} <= {}",
            output_reserve,
            output.amount
        );

        let input_amount = match self.pool_type {
            PassiveLiquidity::Xyk { .. } => {
                // Apply swap fee. In SwapExactIn we multiply ask by (1 - fee) to get the
                // offer amount after fees. So in this case we need to divide ask by (1 - fee)
                // to get the ask amount after fees.
                // Round so that user takes the loss.
                let output_amount_before_fee = output
                    .amount
                    .checked_div_dec_ceil(Udec128::ONE - self.swap_fee_rate.into_inner())?;

                // Solve A * B = (A + input_amount) * (B - output_amount) for input_amount
                // => input_amount = (A * B) / (B - output_amount) - A
                // Round so that user takes the loss.
                Uint128::ONE
                    .checked_multiply_ratio_floor(
                        input_reserve.checked_mul(output_reserve)?,
                        output_reserve.checked_sub(output_amount_before_fee)?,
                    )?
                    .checked_sub(input_reserve)?
            },
            PassiveLiquidity::Geometric { .. } => {
                // TODO: implement
                todo!()
            },
        };

        let input = Coin {
            denom: input_denom,
            amount: input_amount,
        };

        reserve.checked_add(&input)?.checked_sub(&output)?;

        Ok((reserve, input))
    }

    fn reflect_curve(
        self,
        base_denom: Denom,
        quote_denom: Denom,
        reserve: &CoinPair,
    ) -> StdResult<(
        Box<dyn Iterator<Item = (Udec128, Uint128)>>,
        Box<dyn Iterator<Item = (Udec128, Uint128)>>,
    )> {
        let mut base_reserve = reserve.amount_of(&base_denom)?;
        let mut quote_reserve = reserve.amount_of(&quote_denom)?;

        // Compute the marginal price. We will place orders above and below this price.
        let marginal_price = Udec128::checked_from_ratio(quote_reserve, base_reserve)?;

        let swap_fee_rate = self.swap_fee_rate.into_inner();
        let one_plus_fee_rate = Udec128::ONE.checked_add(swap_fee_rate)?;
        let one_sub_fee_rate = Udec128::ONE.checked_sub(swap_fee_rate)?;

        match self.pool_type {
            PassiveLiquidity::Xyk { order_spacing } => {
                // Construct the bid order iterator.
                // Start from the marginal price minus the swap fee rate.
                let mut maybe_price = marginal_price.checked_mul(one_sub_fee_rate).ok();
                let mut prev_size = Uint128::ZERO;
                let mut prev_size_quote = Uint128::ZERO;
                let bids = iter::from_fn(move || {
                    // Terminate if price is less or equal to zero.
                    let price = match maybe_price {
                        Some(price) if price.is_non_zero() => price,
                        _ => return None,
                    };

                    // Compute the total order size (in base asset) at this price.
                    let quote_reserve_div_price = quote_reserve.checked_div_dec(price).ok()?;
                    let mut size = quote_reserve_div_price.checked_sub(base_reserve).ok()?;

                    // Compute the order size (in base asset) at this price.
                    //
                    // This is the difference between the total order size at
                    // this price, and that at the previous price.
                    let mut amount = size.checked_sub(prev_size).ok()?;

                    // Compute the total order size (in quote asset) at this price.
                    let mut amount_quote = amount.checked_mul_dec_ceil(price).ok()?;
                    let mut size_quote = prev_size_quote.checked_add(amount_quote).ok()?;

                    // If total order size (in quote asset) is greater than the
                    // reserve, cap it to the reserve size.
                    if size_quote > quote_reserve {
                        size_quote = quote_reserve;
                        amount_quote = size_quote.checked_sub(prev_size_quote).ok()?;
                        amount = amount_quote.checked_div_dec_floor(price).ok()?;
                        size = prev_size.checked_add(amount).ok()?;
                    }

                    // If order size is zero, we have ran out of liquidity.
                    // Terminate the iterator.
                    if amount.is_zero() {
                        return None;
                    }

                    // Update the iterator state.
                    prev_size = size;
                    prev_size_quote = size_quote;
                    maybe_price = price.checked_sub(order_spacing).ok();

                    Some((price, amount))
                });

                // Construct the ask order iterator.
                let one_plus_fee_rate = Udec128::ONE.checked_add(swap_fee_rate)?;
                let mut maybe_price = marginal_price.checked_mul(one_plus_fee_rate).ok();
                let mut prev_size = Uint128::ZERO;
                let asks = iter::from_fn(move || {
                    let price = maybe_price?;

                    // Compute the total order size (in base asset) at this price.
                    let quote_reserve_div_price = quote_reserve.checked_div_dec(price).ok()?;
                    let size = base_reserve.checked_sub(quote_reserve_div_price).ok()?;

                    // If total order size (in base asset) exceeds the base asset
                    // reserve, cap it to the reserve size.
                    let size = cmp::min(size, base_reserve);

                    // Compute the order size (in base asset) at this price.
                    //
                    // This is the difference between the total order size at
                    // this price, and that at the previous price.
                    let amount = size.checked_sub(prev_size).ok()?;

                    // If order size is zero, we have ran out of liquidity.
                    // Terminate the iterator.
                    if amount.is_zero() {
                        return None;
                    }

                    // Update the iterator state.
                    prev_size = size;
                    maybe_price = price.checked_add(order_spacing).ok();

                    Some((price, amount))
                });

                Ok((Box::new(bids), Box::new(asks)))
            },
            PassiveLiquidity::Geometric {
                ratio,
                order_spacing,
            } => {
                // Construct bid price iterator with decreasing prices
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

                // Iteratively assign `ratio` of the remaining liquidity to each
                // consecutive order.
                let bid_sizes_in_quote = std::iter::from_fn(move || {
                    let size = match quote_reserve.checked_mul_dec(ratio.into_inner()) {
                        Ok(size) => size,
                        Err(_) => return None,
                    };
                    quote_reserve.checked_sub_assign(size).ok()?;
                    Some(size)
                });

                // Zip sizes with prices and convert to each size to base asset size at
                // the price.
                let bids =
                    bid_prices
                        .zip(bid_sizes_in_quote)
                        .filter_map(|(price, size_in_quote)| {
                            let size = size_in_quote.checked_div_dec_floor(price).ok()?;
                            Some((price, size))
                        });

                // Construct ask price iterator with increasing prices
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
                let ask_sizes = std::iter::from_fn(move || {
                    let size = match base_reserve.checked_mul_dec(ratio.into_inner()) {
                        Ok(size) => size,
                        Err(_) => return None,
                    };
                    base_reserve.checked_sub_assign(size).ok()?;
                    Some(size)
                });

                // Zip sizes with prices
                let asks = ask_prices.zip(ask_sizes);

                Ok((Box::new(bids), Box::new(asks)))
            },
        }
    }
}

fn xyk_add_initial_liquidity(
    mut reserve: CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<(CoinPair, Uint128)> {
    reserve.merge(deposit)?;

    // TODO: apply a scaling factor? e.g. 1,000,000 LP tokens per unit of invariant.
    let mint_amount = xyk_normalized_invariant(&reserve)?;

    Ok((reserve, mint_amount))
}

// TODO: use oracle price
fn geometric_add_initial_liquidity(
    mut reserve: CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<(CoinPair, Uint128)> {
    reserve.merge(deposit)?;

    // TODO: apply a scaling factor? e.g. 1,000,000 LP tokens per unit of invariant.
    let mint_amount = xyk_normalized_invariant(&reserve)?;

    Ok((reserve, mint_amount))
}

fn xyk_add_subsequent_liquidity(
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec128> {
    let invariant_before = xyk_normalized_invariant(&reserve)?;

    // Add the used funds to the pool reserves.
    reserve.merge(deposit)?;

    // Compute the proportional increase in the invariant.
    let invariant_after = xyk_normalized_invariant(&reserve)?;
    let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

    // Compute the mint ratio from the invariant ratio based on the curve type.
    // This ensures that an unbalances provision will be equivalent to a swap
    // followed by a balancedliquidity provision.
    Ok(invariant_ratio.checked_sub(Udec128::ONE)?)
}

fn geometric_add_subsequent_liquidity(
    oracle_querier: &mut OracleQuerier,
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec128> {
    fn oracle_value(
        oracle_querier: &mut OracleQuerier,
        coin_pair: &CoinPair,
    ) -> anyhow::Result<Udec128> {
        let first_value = oracle_querier
            .query_price(coin_pair.first().denom, None)?
            .value_of_unit_amount(*coin_pair.first().amount)?;
        let second_value = oracle_querier
            .query_price(coin_pair.second().denom, None)?
            .value_of_unit_amount(*coin_pair.second().amount)?;
        Ok(first_value.checked_add(second_value)?)
    }

    let deposit_value = oracle_value(oracle_querier, &deposit)?;
    let reserve_value = oracle_value(oracle_querier, &reserve)?;

    Ok(deposit_value.checked_div(reserve_value.checked_add(deposit_value)?)?)
}

/// Compute `sqrt(A * B)`, where `A` and `B` are the reserve amount of the two
/// assets in an xyk pool.
fn xyk_normalized_invariant(reserve: &CoinPair) -> MathResult<Uint128> {
    let a = *reserve.first().amount;
    let b = *reserve.second().amount;

    a.checked_mul(b)?.checked_sqrt()
}

/// Compute `|a - b|`.
fn abs_diff(a: Uint128, b: Uint128) -> Uint128 {
    if a > b {
        a - b
    } else {
        b - a
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::constants::{eth, usdc},
        grug::{Bounded, Coins, coins},
        test_case::test_case,
    };

    #[test_case(
        PassiveLiquidity::Xyk {
            order_spacing: Udec128::ONE,
        },
        Udec128::new_permille(5),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 200 * 10000000,
        },
        vec![
            (Udec128::new_percent(19900), Uint128::from(50251)),
            (Udec128::new_percent(19800), Uint128::from(50759)),
            (Udec128::new_percent(19700), Uint128::from(51274)),
            (Udec128::new_percent(19600), Uint128::from(51797)),
            (Udec128::new_percent(19500), Uint128::from(52329)),
            (Udec128::new_percent(19400), Uint128::from(52868)),
            (Udec128::new_percent(19300), Uint128::from(53416)),
            (Udec128::new_percent(19200), Uint128::from(53972)),
            (Udec128::new_percent(19100), Uint128::from(54538)),
            (Udec128::new_percent(19000), Uint128::from(55112)),
        ],
        vec![
            (Udec128::new_percent(20100), Uint128::from(49751)),
            (Udec128::new_percent(20200), Uint128::from(49259)),
            (Udec128::new_percent(20300), Uint128::from(48773)),
            (Udec128::new_percent(20400), Uint128::from(48295)),
            (Udec128::new_percent(20500), Uint128::from(47824)),
            (Udec128::new_percent(20600), Uint128::from(47360)),
            (Udec128::new_percent(20700), Uint128::from(46902)),
            (Udec128::new_percent(20800), Uint128::from(46451)),
            (Udec128::new_percent(20900), Uint128::from(46007)),
            (Udec128::new_percent(21000), Uint128::from(45568)),
        ],
        1;
        "xyk pool balance 1:200 tick size 1 0.5% fee"
    )]
    #[test_case(
        PassiveLiquidity::Xyk {
            order_spacing: Udec128::ONE,
        },
        Udec128::new_percent(1),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 200 * 10000000,
        },
        vec![
            (Udec128::new_percent(19800), Uint128::from(101010)),
            (Udec128::new_percent(19700), Uint128::from(51274)),
            (Udec128::new_percent(19600), Uint128::from(51797)),
            (Udec128::new_percent(19500), Uint128::from(52329)),
            (Udec128::new_percent(19400), Uint128::from(52868)),
            (Udec128::new_percent(19300), Uint128::from(53416)),
            (Udec128::new_percent(19200), Uint128::from(53972)),
            (Udec128::new_percent(19100), Uint128::from(54538)),
            (Udec128::new_percent(19000), Uint128::from(55112)),
            (Udec128::new_percent(18900), Uint128::from(55694)),
        ],
        vec![
            (Udec128::new_percent(20200), Uint128::from(99010)),
            (Udec128::new_percent(20300), Uint128::from(48774)),
            (Udec128::new_percent(20400), Uint128::from(48295)),
            (Udec128::new_percent(20500), Uint128::from(47824)),
            (Udec128::new_percent(20600), Uint128::from(47360)),
            (Udec128::new_percent(20700), Uint128::from(46902)),
            (Udec128::new_percent(20800), Uint128::from(46451)),
            (Udec128::new_percent(20900), Uint128::from(46007)),
            (Udec128::new_percent(21000), Uint128::from(45568)),
            (Udec128::new_percent(21100), Uint128::from(45137)),
        ],
        1;
        "xyk pool balance 1:200 tick size 1 one percent fee"
    )]
    #[test_case(
        PassiveLiquidity::Xyk {
            order_spacing: Udec128::new_percent(1),
        },
        Udec128::new_permille(5),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        vec![
            (Udec128::new_permille(995), Uint128::from(50251)),
            (Udec128::new_permille(985), Uint128::from(102033)),
            (Udec128::new_permille(975), Uint128::from(104126)),
            (Udec128::new_permille(965), Uint128::from(106284)),
            (Udec128::new_permille(955), Uint128::from(108510)),
            (Udec128::new_permille(945), Uint128::from(110806)),
            (Udec128::new_permille(935), Uint128::from(113177)),
            (Udec128::new_permille(925), Uint128::from(115624)),
            (Udec128::new_permille(915), Uint128::from(118151)),
            (Udec128::new_permille(905), Uint128::from(120762)),
        ],
        vec![
            (Udec128::new_permille(1005), Uint128::from(49751)),
            (Udec128::new_permille(1015), Uint128::from(98032)),
            (Udec128::new_permille(1025), Uint128::from(96119)),
            (Udec128::new_permille(1035), Uint128::from(94262)),
            (Udec128::new_permille(1045), Uint128::from(92458)),
            (Udec128::new_permille(1055), Uint128::from(90705)),
            (Udec128::new_permille(1065), Uint128::from(89002)),
            (Udec128::new_permille(1075), Uint128::from(87346)),
            (Udec128::new_permille(1085), Uint128::from(85736)),
            (Udec128::new_permille(1095), Uint128::from(84170)),
        ],
        1;
        "xyk pool balance 1:1 0.5% fee"
    )]
    #[test_case(
        PassiveLiquidity::Xyk {
            order_spacing: Udec128::new_percent(1),
        },
        Udec128::new_percent(1),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        vec![
            (Udec128::new_percent(99), Uint128::from(101010)),
            (Udec128::new_percent(98), Uint128::from(103072)),
            (Udec128::new_percent(97), Uint128::from(105196)),
            (Udec128::new_percent(96), Uint128::from(107388)),
            (Udec128::new_percent(95), Uint128::from(109649)),
            (Udec128::new_percent(94), Uint128::from(111982)),
            (Udec128::new_percent(93), Uint128::from(114390)),
            (Udec128::new_percent(92), Uint128::from(116877)),
            (Udec128::new_percent(91), Uint128::from(119445)),
            (Udec128::new_percent(90), Uint128::from(122100)),
        ],
        vec![
            (Udec128::new_percent(101), Uint128::from(99010)),
            (Udec128::new_percent(102), Uint128::from(97070)),
            (Udec128::new_percent(103), Uint128::from(95184)),
            (Udec128::new_percent(104), Uint128::from(93353)),
            (Udec128::new_percent(105), Uint128::from(91575)),
            (Udec128::new_percent(106), Uint128::from(89847)),
            (Udec128::new_percent(107), Uint128::from(88168)),
            (Udec128::new_percent(108), Uint128::from(86535)),
            (Udec128::new_percent(109), Uint128::from(84947)),
            (Udec128::new_percent(110), Uint128::from(83403)),
        ],
        1;
        "xyk pool balance 1:1 one percent fee"
    )]
    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(70)).unwrap(),
            order_spacing: Udec128::new_percent(1),
        },
        Udec128::new_percent(1),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        vec![
            (Udec128::new_percent(99), Uint128::from(7070707)),
            (Udec128::new_percent(98), Uint128::from(2142857)),
            (Udec128::new_percent(97), Uint128::from(649484)),
            (Udec128::new_percent(96), Uint128::from(196875)),
            (Udec128::new_percent(95), Uint128::from(59684)),
            (Udec128::new_percent(94), Uint128::from(18095)),
            (Udec128::new_percent(93), Uint128::from(5487)),
            (Udec128::new_percent(92), Uint128::from(1663)),
            (Udec128::new_percent(91), Uint128::from(504)),
            (Udec128::new_percent(90), Uint128::from(152)),
        ],
        vec![
            (Udec128::new_percent(101), Uint128::from(7000000)),
            (Udec128::new_percent(102), Uint128::from(2100000)),
            (Udec128::new_percent(103), Uint128::from(630000)),
            (Udec128::new_percent(104), Uint128::from(189000)),
            (Udec128::new_percent(105), Uint128::from(56700)),
            (Udec128::new_percent(106), Uint128::from(17010)),
            (Udec128::new_percent(107), Uint128::from(5103)),
            (Udec128::new_percent(108), Uint128::from(1530)),
            (Udec128::new_percent(109), Uint128::from(459)),
            (Udec128::new_percent(110), Uint128::from(137)),
        ],
        1;
        "geometric pool balance 1:1 30% ratio"
    )]
    fn curve_on_orderbook(
        pool_type: PassiveLiquidity,
        swap_fee_rate: Udec128,
        pool_liquidity: Coins,
        expected_bids: Vec<(Udec128, Uint128)>,
        expected_asks: Vec<(Udec128, Uint128)>,
        order_size_tolerance: u128,
    ) {
        let pair = PairParams {
            pool_type,
            swap_fee_rate: Bounded::new(swap_fee_rate).unwrap(),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        let reserve = pool_liquidity.try_into().unwrap();
        let (bids, asks) = pair
            .reflect_curve(eth::DENOM.clone(), usdc::DENOM.clone(), &reserve)
            .unwrap();

        // Assert that at least 10 orders are returned.
        let bids = bids.take(expected_bids.len()).collect::<Vec<_>>();
        let asks = asks.take(expected_asks.len()).collect::<Vec<_>>();
        assert_eq!(bids.len(), expected_bids.len());
        assert_eq!(asks.len(), expected_asks.len());

        // Assert that the orders are correct.
        for (ask, expected_ask) in asks.into_iter().zip(expected_asks.iter()) {
            assert_eq!(ask.0, expected_ask.0);
            assert!(
                ask.1.into_inner().abs_diff(expected_ask.1.into_inner()) <= order_size_tolerance
            );
        }

        for (bid, expected_bid) in bids.into_iter().zip(expected_bids.iter()) {
            assert_eq!(bid.0, expected_bid.0);
            assert!(
                bid.1.into_inner().abs_diff(expected_bid.1.into_inner()) <= order_size_tolerance
            );
        }
    }

    #[test]
    fn geometric_pool_iterator_stops_at_zero_price() {
        let pair = PairParams {
            pool_type: PassiveLiquidity::Geometric {
                ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
                order_spacing: Udec128::new_percent(50),
            },
            swap_fee_rate: Bounded::new(Udec128::new_percent(1)).unwrap(),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        let reserve = coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        }
        .try_into()
        .unwrap();

        let (bids, asks) = pair
            .reflect_curve(eth::DENOM.clone(), usdc::DENOM.clone(), &reserve)
            .unwrap();

        let bids_collected = bids.collect::<Vec<_>>();

        assert_eq!(bids_collected.len(), 2);

        for (bid, expected_bid) in bids_collected.into_iter().zip(vec![
            (Udec128::new_percent(99), Uint128::from(5050505)),
            (Udec128::new_percent(49), Uint128::from(5102040)),
        ]) {
            assert_eq!(bid.0, expected_bid.0);
            assert_eq!(bid.1, expected_bid.1);
        }

        // Check that ask iterator keeps going after bid iterator is exhausted
        let asks_collected = asks.take(10).collect::<Vec<_>>();
        assert_eq!(asks_collected.len(), 10);
    }
}
