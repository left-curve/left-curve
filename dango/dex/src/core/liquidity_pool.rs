use {
    crate::TradingFunction,
    anyhow::ensure,
    dango_types::dex::{CurveInvariant, PairParams},
    grug::{
        Coin, CoinPair, Denom, Inner, IsZero, MultiplyFraction, MultiplyRatio, Number, NumberConst,
        StdResult, Udec128, Uint128,
    },
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
    /// - `reserves`: The current pool reserves.
    /// - `spread`: The spread between the ask and bid.
    ///
    /// ## Outputs
    ///
    /// - A tuple of two vectors of orders to place on the orderbook. The first vector
    ///   contains the bids, the second contains the asks, specified as a tuple of
    ///   (price, amount).
    fn reflect_curve(
        &self,
        base_denom: Denom,
        quote_denom: Denom,
        reserves: &CoinPair,
    ) -> StdResult<(Vec<(Udec128, Uint128)>, Vec<(Udec128, Uint128)>)>;

    /// Returns the spot price of the pool at the current reserves, given as the number
    /// of quote asset units per base asset unit.
    ///
    /// ## Inputs
    ///
    /// - `base_denom`: The base asset of the pool.
    /// - `quote_denom`: The quote asset of the pool.
    /// - `reserves`: The current pool reserves.
    ///
    /// ## Outputs
    ///
    /// - The spot price of the pool.
    fn spot_price(
        &self,
        base_denom: &Denom,
        quote_denom: &Denom,
        reserves: &CoinPair,
    ) -> StdResult<Udec128>;
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
            CurveInvariant::Xyk => {
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
            CurveInvariant::Xyk => {
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
        &self,
        base_denom: Denom,
        quote_denom: Denom,
        reserves: &CoinPair,
    ) -> StdResult<(Vec<(Udec128, Uint128)>, Vec<(Udec128, Uint128)>)> {
        let base_reserves = reserves.amount_of(&base_denom)?;
        let quote_reserves = reserves.amount_of(&quote_denom)?;

        let price = self.spot_price(&base_denom, &quote_denom, reserves)?;

        // Calculate the starting price for the ask and bid side respectively. We
        // place the spread symmetrically around the spot price.
        let swap_fee_rate = self.swap_fee_rate.into_inner();
        let starting_price_ask = price.checked_mul(Udec128::ONE.checked_add(swap_fee_rate)?)?;
        let starting_price_bid = price.checked_mul(Udec128::ONE.checked_sub(swap_fee_rate)?)?;

        let mut bids = Vec::with_capacity(self.order_depth as usize);
        let mut asks = Vec::with_capacity(self.order_depth as usize);
        let mut amount_bid_prev = Uint128::ZERO;
        let mut amount_ask_prev = Uint128::ZERO;
        for i in 1..(self.order_depth + 1) {
            // Calculate the price that is `i` price steps of size `order_spacing`
            // on the ask and bid side of the spot price respectively.
            let delta_p = self
                .order_spacing
                .checked_mul(Udec128::checked_from_ratio(i as u128, Uint128::ONE)?)?;
            let price_ask = starting_price_ask.checked_add(delta_p)?;
            let price_bid = starting_price_bid.checked_sub(delta_p)?;

            // Calculate the amount of base that the pool will trade from the
            // current spot price to `price_ask` and `price_bid` respectively.
            let (amount_ask, amount_bid) = match self.curve_invariant {
                CurveInvariant::Xyk => {
                    let amount_ask =
                        base_reserves.checked_sub(quote_reserves.checked_div_dec(price_ask)?)?;
                    let amount_bid = quote_reserves
                        .checked_div_dec(price_bid)?
                        .checked_sub(base_reserves)?;
                    (amount_ask, amount_bid)
                },
            };

            // Calculate the difference in the amount as compared to the previous iteration.
            // I.e. the amount that the pool would trade between the previous price and the
            // current price, which is the amount of the order to place at the current price.
            // Only place orders if the amount is positive.
            let amount_bid_diff = amount_bid.checked_sub(amount_bid_prev)?;
            if amount_bid_diff > Uint128::ZERO {
                bids.push((price_bid, amount_bid_diff));
            }

            let amount_ask_diff = amount_ask.checked_sub(amount_ask_prev)?;
            if amount_ask_diff > Uint128::ZERO {
                asks.push((price_ask, amount_ask_diff));
            }

            // Update the previous amounts for the next iteration.
            amount_bid_prev = amount_bid;
            amount_ask_prev = amount_ask;
        }

        Ok((bids, asks))
    }

    fn spot_price(
        &self,
        base_denom: &Denom,
        quote_denom: &Denom,
        reserves: &CoinPair,
    ) -> StdResult<Udec128> {
        let base_reserves = reserves.amount_of(base_denom)?;
        let quote_reserves = reserves.amount_of(quote_denom)?;

        match self.curve_invariant {
            CurveInvariant::Xyk => Ok(Udec128::checked_from_ratio(quote_reserves, base_reserves)?),
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

#[cfg(test)]
mod tests {

    use {
        dango_types::constants::{eth, usdc},
        grug::{Bounded, coins},
    };

    use super::*;

    use {grug::Coins, test_case::test_case};

    #[test_case(
        CurveInvariant::Xyk,
        Udec128::ONE,
        10,
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
        1 ; "xyk pool balance 1:200 tick size 1 no fee")]
    #[test_case(
        CurveInvariant::Xyk,
        Udec128::ONE,
        10,
        Udec128::new_percent(1),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 200 * 10000000,
        },
        vec![
            (Udec128::new_percent(19700), Uint128::from(152284)),
            (Udec128::new_percent(19600), Uint128::from(51797)),
            (Udec128::new_percent(19500), Uint128::from(52329)),
            (Udec128::new_percent(19400), Uint128::from(52868)),
            (Udec128::new_percent(19300), Uint128::from(53416)),
            (Udec128::new_percent(19200), Uint128::from(53972)),
            (Udec128::new_percent(19100), Uint128::from(54538)),
            (Udec128::new_percent(19000), Uint128::from(55112)),
            (Udec128::new_percent(18900), Uint128::from(55694)),
            (Udec128::new_percent(18800), Uint128::from(56287)),
        ],
        vec![
            (Udec128::new_percent(20300), Uint128::from(147783)),
            (Udec128::new_percent(20400), Uint128::from(48295)),
            (Udec128::new_percent(20500), Uint128::from(47824)),
            (Udec128::new_percent(20600), Uint128::from(47360)),
            (Udec128::new_percent(20700), Uint128::from(46902)),
            (Udec128::new_percent(20800), Uint128::from(46451)),
            (Udec128::new_percent(20900), Uint128::from(46007)),
            (Udec128::new_percent(21000), Uint128::from(45568)),
            (Udec128::new_percent(21100), Uint128::from(45137)),
            (Udec128::new_percent(21200), Uint128::from(44711)),
        ],
        1 ; "xyk pool balance 1:200 tick size 1 one percent fee")]
    #[test_case(
        CurveInvariant::Xyk,
        Udec128::new_percent(1),
        10,
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
        1 ; "xyk pool balance 1:1 no fee")]
    #[test_case(
        CurveInvariant::Xyk,
        Udec128::new_percent(1),
        10,
        Udec128::new_percent(1),
        coins! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        vec![
            (Udec128::new_percent(98), Uint128::from(204081)),
            (Udec128::new_percent(97), Uint128::from(105196)),
            (Udec128::new_percent(96), Uint128::from(107388)),
            (Udec128::new_percent(95), Uint128::from(109649)),
            (Udec128::new_percent(94), Uint128::from(111982)),
            (Udec128::new_percent(93), Uint128::from(114390)),
            (Udec128::new_percent(92), Uint128::from(116877)),
            (Udec128::new_percent(91), Uint128::from(119445)),
            (Udec128::new_percent(90), Uint128::from(122100)),
            (Udec128::new_percent(89), Uint128::from(124843)),
        ],
        vec![
            (Udec128::new_percent(102), Uint128::from(196078)),
            (Udec128::new_percent(103), Uint128::from(95184)),
            (Udec128::new_percent(104), Uint128::from(93353)),
            (Udec128::new_percent(105), Uint128::from(91575)),
            (Udec128::new_percent(106), Uint128::from(89847)),
            (Udec128::new_percent(107), Uint128::from(88168)),
            (Udec128::new_percent(108), Uint128::from(86535)),
            (Udec128::new_percent(109), Uint128::from(84947)),
            (Udec128::new_percent(110), Uint128::from(83403)),
            (Udec128::new_percent(111), Uint128::from(81900)),
        ],
        1 ; "xyk pool balance 1:1 one percent fee")]
    fn curve_on_orderbook(
        curve_invariant: CurveInvariant,
        order_spacing: Udec128,
        order_depth: u64,
        swap_fee_rate: Udec128,
        pool_liquidity: Coins,
        expected_bids: Vec<(Udec128, Uint128)>,
        expected_asks: Vec<(Udec128, Uint128)>,
        order_size_tolerance: u128,
    ) {
        let pair = PairParams {
            curve_invariant,
            order_spacing,
            order_depth,
            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        let (bids, asks) = pair
            .reflect_curve(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                &pool_liquidity.try_into().unwrap(),
            )
            .unwrap();

        for (bid, expected_bid) in bids.iter().zip(expected_bids.iter()) {
            assert_eq!(bid.0, expected_bid.0);
            assert!(
                bid.1.into_inner().abs_diff(expected_bid.1.into_inner()) <= order_size_tolerance
            );
        }

        for (ask, expected_ask) in asks.iter().zip(expected_asks.iter()) {
            assert_eq!(ask.0, expected_ask.0);
            assert!(
                ask.1.into_inner().abs_diff(expected_ask.1.into_inner()) <= order_size_tolerance
            );
        }
    }
}
