use {
    crate::TradingFunction,
    anyhow::ensure,
    dango_types::dex::{CurveInvariant, PairParams},
    grug::{
        Coin, CoinPair, Denom, Inner, IsZero, MultiplyFraction, MultiplyRatio, Number, NumberConst,
        StdResult, Udec128, Uint128,
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
        reserve: CoinPair,
        lp_token_supply: Uint128,
        lp_burn_amount: Uint128,
    ) -> anyhow::Result<(CoinPair, CoinPair)>;

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
        mut reserve: CoinPair,
        lp_token_supply: Uint128,
        deposit: CoinPair,
    ) -> anyhow::Result<(CoinPair, Uint128)> {
        if lp_token_supply.is_zero() {
            reserve.merge(deposit.clone())?;

            let invariant = self.curve_invariant.normalized_invariant(&reserve)?;

            // TODO: apply a scaling factor? e.g. 1,000,000 LP tokens per unit of invariant.
            let mint_amount = invariant;

            Ok((reserve, mint_amount))
        } else {
            let invariant_before = self.curve_invariant.normalized_invariant(&reserve)?;

            // Add the used funds to the pool reserves.
            reserve.merge(deposit.clone())?;

            // Compute the proportional increase in the invariant.
            let invariant_after = self.curve_invariant.normalized_invariant(&reserve)?;
            let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

            // Compute the mint ratio from the invariant ratio based on the curve type.
            // This ensures that an unbalances provision will be equivalent to a swap
            // followed by a balancedliquidity provision.
            let mint_ratio = invariant_ratio.checked_sub(Udec128::ONE)?;
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

            let mint_amount = mint_amount_before_fee
                .checked_mul_dec_floor(Udec128::ONE.checked_sub(fee_rate)?)?;

            Ok((reserve, mint_amount))
        }
    }

    fn remove_liquidity(
        &self,
        mut reserve: CoinPair,
        lp_token_supply: Uint128,
        lp_burn_amount: Uint128,
    ) -> anyhow::Result<(CoinPair, CoinPair)> {
        let refund = reserve.split(lp_burn_amount, lp_token_supply)?;

        Ok((reserve, refund))
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

        let output_amount_after_fee = match self.curve_invariant {
            CurveInvariant::Xyk { .. } => {
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

        let input_amount = match self.curve_invariant {
            CurveInvariant::Xyk { .. } => {
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
        let base_reserve = reserve.amount_of(&base_denom)?;
        let quote_reserve = reserve.amount_of(&quote_denom)?;

        // Compute the marginal price. We will place orders above and below this price.
        let marginal_price = Udec128::checked_from_ratio(quote_reserve, base_reserve)?;

        match self.curve_invariant {
            CurveInvariant::Xyk { order_spacing } => {
                let swap_fee_rate = self.swap_fee_rate.into_inner();

                // Construct the bid order iterator.
                // Start from the marginal price minus the swap fee rate.
                let one_sub_fee_rate = Udec128::ONE.checked_sub(swap_fee_rate)?;
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
        }
    }
}

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
        CurveInvariant::Xyk {
            order_spacing: Udec128::ONE,
        },
        Udec128::ZERO,
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
        "xyk pool balance 1:200 tick size 1 no fee"
    )]
    #[test_case(
        CurveInvariant::Xyk {
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
        CurveInvariant::Xyk {
            order_spacing: Udec128::new_percent(1),
        },
        Udec128::ZERO,
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        vec![
            (Udec128::new_percent(99), Uint128::from(101010)),
            (Udec128::new_percent(98), Uint128::from(103072)),
            (Udec128::new_percent(97), Uint128::from(105197)),
            (Udec128::new_percent(96), Uint128::from(107388)),
            (Udec128::new_percent(95), Uint128::from(109649)),
            (Udec128::new_percent(94), Uint128::from(111982)),
            (Udec128::new_percent(93), Uint128::from(114390)),
            (Udec128::new_percent(92), Uint128::from(116877)),
            (Udec128::new_percent(91), Uint128::from(119446)),
            (Udec128::new_percent(90), Uint128::from(122100)),
        ],
        vec![
            (Udec128::new_percent(101), Uint128::from(99010)),
            (Udec128::new_percent(102), Uint128::from(97069)),
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
        "xyk pool balance 1:1 no fee"
    )]
    #[test_case(
        CurveInvariant::Xyk {
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
    fn curve_on_orderbook(
        curve_invariant: CurveInvariant,
        swap_fee_rate: Udec128,
        pool_liquidity: Coins,
        expected_bids: Vec<(Udec128, Uint128)>,
        expected_asks: Vec<(Udec128, Uint128)>,
        order_size_tolerance: u128,
    ) {
        let pair = PairParams {
            curve_invariant,
            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        let reserve = pool_liquidity.try_into().unwrap();
        let (bids, asks) = pair
            .reflect_curve(eth::DENOM.clone(), usdc::DENOM.clone(), &reserve)
            .unwrap();

        for (bid, expected_bid) in bids.zip(expected_bids.iter()) {
            assert_eq!(bid.0, expected_bid.0);
            assert!(
                bid.1.into_inner().abs_diff(expected_bid.1.into_inner()) <= order_size_tolerance
            );
        }

        for (ask, expected_ask) in asks.zip(expected_asks.iter()) {
            assert_eq!(ask.0, expected_ask.0);
            assert!(
                ask.1.into_inner().abs_diff(expected_ask.1.into_inner()) <= order_size_tolerance
            );
        }
    }
}
