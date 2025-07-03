use {
    crate::{
        PassiveOrder,
        core::{geometric, xyk},
    },
    anyhow::{bail, ensure},
    dango_oracle::OracleQuerier,
    dango_types::dex::{PairParams, PassiveLiquidity},
    grug::{
        Coin, CoinPair, Denom, IsZero, MultiplyFraction, Number, NumberConst, Udec128, Uint128,
    },
    std::ops::Sub,
};

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
        oracle_querier: &mut OracleQuerier,
        base_denom: &Denom,
        quote_denom: &Denom,
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
        oracle_querier: &mut OracleQuerier,
        base_denom: &Denom,
        quote_denom: &Denom,
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
    ///   tuple of (price, passive_order).
    ///
    /// ## Notes
    ///
    /// Note that the iterator item doesn't a `Result` type. If there is an
    /// error in computing the order, the iterator should return `None` and thus
    /// terminates.
    fn reflect_curve(
        self,
        oracle_querier: &mut OracleQuerier,
        base_denom: Denom,
        quote_denom: Denom,
        reserve: &CoinPair,
    ) -> anyhow::Result<(
        Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>, // bids
        Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>, // asks
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
        // The deposit must have the same denoms as the reserve. This should
        // have been caught earlier in `execute::provide_liquidity`.
        // We assert this in debug builds only.
        #[cfg(debug_assertions)]
        {
            let deposit_denoms = (deposit.first().denom, deposit.second().denom);
            let reserve_denoms = (reserve.first().denom, reserve.second().denom);
            ensure!(
                deposit_denoms == reserve_denoms,
                "deposit denoms {deposit_denoms:?} don't match reserve denoms {reserve_denoms:?}",
            );
        }

        // If there isn't any liquidity in the pool yet, run the special logic
        // for adding initial liquidity, then early return.
        if lp_token_supply.is_zero() {
            let mint_amount = match &self.pool_type {
                PassiveLiquidity::Xyk { .. } => xyk::add_initial_liquidity(&deposit)?,
                PassiveLiquidity::Geometric { .. } => {
                    geometric::add_initial_liquidity(oracle_querier, &deposit)?
                },
            };

            reserve.merge(deposit.clone())?;

            return Ok((reserve, mint_amount));
        }

        let mint_ratio = match &self.pool_type {
            PassiveLiquidity::Xyk { .. } => {
                xyk::add_subsequent_liquidity(&mut reserve, deposit.clone())?
            },
            PassiveLiquidity::Geometric { .. } => {
                geometric::add_subsequent_liquidity(oracle_querier, &mut reserve, deposit.clone())?
            },
        };

        // In case the deposit is asymmetrical, we need to apply a deposit fee.
        //
        // This is to prevent an attack where a user deposits asymmetrically,
        // then immediately withdraw symmetrically, essentially accomplishing a
        // swap without paying the swap fee.
        //
        // We determine the deposit fee rate based oracle price as follows:
        //
        // - Suppose the pool's reserve is `A` dollars of the 1st asset and `B`
        //   dollars of the 2nd asset.
        // - Suppose a user deposits `a` dollars of 1st asset and `b` dollars of
        //   the 2nd asset.
        // - Note that `A`, `B`, `a`, and `b` here are values in USD, not the
        //   unit amounts.
        // - Without losing generality, assume a / A > b / B. In this case, the
        //   user is over-supplying the 1st asset and under-supplying the 2nd
        //   asset. To make the deposit symmetrical, the user needs to swap some
        //   1st asset into some 2nd asset.
        // - Suppose user swap `x` dollars of the 1st asset into the 2nd asset.
        //   Also suppose our pool does the swap at exactly the oracle price
        //   without slippage. This assumption obviously isn't true, but is good
        //   enough for the purpose of preventing the aforementioned attack.
        // - We must solve:
        //   (a - x) / (A + x) = (b + x) / (B - x)
        //   The solution is:
        //   x = (a * B - A * b) / (a + A + b + B)
        // - We charge a fee assuming this swap is to be carried out. The USD
        //   value of the fee is:
        //   x * swap_fee_rate
        // - To charge the fee, we mint slightly less LP tokens corresponding to
        //   the ratio:
        //   x * swap_fee_rate / (a + b)
        //
        // Related: Curve V2 also applies a fee for asymmetrical deposits:
        // https://github.com/curvefi/twocrypto-ng/blob/main/contracts/main/Twocrypto.vy#L1146-L1168
        // However, their math appears to be only suitable for the Curve V2 curve.
        // Our oracle approach is more generalizable to different pool types.
        let fee_rate = {
            let price = oracle_querier.query_price(reserve.first().denom, None)?;
            let a = price.value_of_unit_amount(*deposit.first().amount)?;
            let reserve_a = price.value_of_unit_amount(*reserve.first().amount)?;

            let price = oracle_querier.query_price(reserve.second().denom, None)?;
            let b = price.value_of_unit_amount(*deposit.second().amount)?;
            let reserve_b = price.value_of_unit_amount(*reserve.second().amount)?;

            let deposit_value = a.checked_add(b)?;
            let reserve_value = reserve_a.checked_add(reserve_b)?;

            abs_diff(a.checked_mul(reserve_b)?, b.checked_mul(reserve_a)?)
                .checked_div(deposit_value.checked_add(reserve_value)?)?
                .checked_mul(*self.swap_fee_rate)?
                .checked_div(deposit_value)?
        };

        let mint_amount = {
            let mint_amount_before_fee = lp_token_supply.checked_mul_dec_floor(mint_ratio)?;
            let one_sub_fee_rate = Udec128::ONE.checked_sub(fee_rate)?;

            mint_amount_before_fee.checked_mul_dec_floor(one_sub_fee_rate)?
        };

        Ok((reserve, mint_amount))
    }

    fn swap_exact_amount_in(
        &self,
        oracle_querier: &mut OracleQuerier,
        base_denom: &Denom,
        quote_denom: &Denom,
        mut reserve: CoinPair,
        input: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)> {
        let output_denom = if reserve.first().denom == &input.denom {
            reserve.second().denom.clone()
        } else if reserve.second().denom == &input.denom {
            reserve.first().denom.clone()
        } else {
            bail!(
                "input denom `{}` is neither the base `{}` nor the quote `{}`",
                input.denom,
                base_denom,
                quote_denom
            );
        };

        let output_amount_after_fee = match self.pool_type {
            PassiveLiquidity::Xyk { .. } => xyk::swap_exact_amount_in(
                input.amount,
                reserve.amount_of(&input.denom)?,
                reserve.amount_of(&output_denom)?,
                self.swap_fee_rate,
            )?,
            PassiveLiquidity::Geometric {
                ratio,
                order_spacing,
            } => geometric::swap_exact_amount_in(
                oracle_querier,
                base_denom,
                quote_denom,
                &input,
                &reserve,
                ratio,
                order_spacing,
                self.swap_fee_rate,
            )?,
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
        oracle_querier: &mut OracleQuerier,
        base_denom: &Denom,
        quote_denom: &Denom,
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
            PassiveLiquidity::Xyk { .. } => xyk::swap_exact_amount_out(
                output.amount,
                input_reserve,
                output_reserve,
                self.swap_fee_rate,
            )?,
            PassiveLiquidity::Geometric {
                ratio,
                order_spacing,
            } => geometric::swap_exact_amount_out(
                oracle_querier,
                base_denom,
                quote_denom,
                &output,
                &reserve,
                ratio,
                order_spacing,
                self.swap_fee_rate,
            )?,
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
        oracle_querier: &mut OracleQuerier,
        base_denom: Denom,
        quote_denom: Denom,
        reserve: &CoinPair,
    ) -> anyhow::Result<(
        Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>,
        Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>,
    )> {
        let base_reserve = reserve.amount_of(&base_denom)?;
        let quote_reserve = reserve.amount_of(&quote_denom)?;

        match self.pool_type {
            PassiveLiquidity::Xyk { order_spacing } => xyk::reflect_curve(
                base_reserve,
                quote_reserve,
                order_spacing,
                self.swap_fee_rate,
            ),
            PassiveLiquidity::Geometric {
                ratio,
                order_spacing,
            } => geometric::reflect_curve(
                oracle_querier,
                &base_denom,
                &quote_denom,
                base_reserve,
                quote_reserve,
                ratio,
                order_spacing,
                self.swap_fee_rate,
            ),
        }
    }
}

/// Compute `|a - b|`.
fn abs_diff<T>(a: T, b: T) -> <T as Sub>::Output
where
    T: Ord + Sub,
{
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
        dango_types::{
            constants::{eth, usdc},
            oracle::PrecisionedPrice,
        },
        grug::{Bounded, Coins, Inner, Timestamp, coin_pair, coins, hash_map},
        std::collections::HashMap,
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

        // Mock the oracle to return a price of 1 with 6 decimals for both assets.
        // TODO: take prices as test parameters
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        });

        let reserve = pool_liquidity.try_into().unwrap();
        let (bids, asks) = pair
            .reflect_curve(
                &mut oracle_querier,
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                &reserve,
            )
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
                ask.1.amount.inner().abs_diff(expected_ask.1.into_inner()) <= order_size_tolerance
            );
        }

        for (bid, expected_bid) in bids.into_iter().zip(expected_bids.iter()) {
            assert_eq!(bid.0, expected_bid.0);
            assert!(
                bid.1.amount.inner().abs_diff(expected_bid.1.into_inner()) <= order_size_tolerance
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

        // Mock the oracle to return a price of 1 with 6 decimals for both assets.
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        });

        let (bids, asks) = pair
            .reflect_curve(
                &mut oracle_querier,
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                &reserve,
            )
            .unwrap();

        let bids_collected = bids.collect::<Vec<_>>();

        assert_eq!(bids_collected.len(), 2);

        for (bid, expected_bid) in bids_collected.into_iter().zip(vec![
            (Udec128::new_percent(99), Uint128::from(5050505)),
            (Udec128::new_percent(49), Uint128::from(5102040)),
        ]) {
            assert_eq!(bid.0, expected_bid.0);
            assert_eq!(bid.1.amount, expected_bid.1);
        }

        // Check that ask iterator keeps going after bid iterator is exhausted
        let asks_collected = asks.take(10).collect::<Vec<_>>();
        assert_eq!(asks_collected.len(), 10);
    }

    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(eth::DENOM.clone(), 5000000).unwrap(),
        Coin::new(usdc::DENOM.clone(), 4900500).unwrap(),
        coin_pair! {
            eth::DENOM.clone() => 10000000 + 5000000,
            usdc::DENOM.clone() => 10000000 - 4900500,
        };
        "geometric pool 1:1 price swap in base denom amount matches first order"
    )]
    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(usdc::DENOM.clone(), 5000000).unwrap(),
        Coin::new(eth::DENOM.clone(), 4900990).unwrap(),
        coin_pair! {
            eth::DENOM.clone() => 10000000 - 4900990,
            usdc::DENOM.clone() => 10000000 + 5000000,
        };
        "geometric pool 1:1 price swap in quote denom amount matches first order"
    )]
    fn swap_exact_amount_in(
        pool_type: PassiveLiquidity,
        reserve: CoinPair,
        oracle_prices: HashMap<Denom, PrecisionedPrice>,
        fee_rate: Udec128,
        input: Coin,
        expected_output: Coin,
        expected_reserve_after_swap: CoinPair,
    ) {
        let pair = PairParams {
            pool_type,
            swap_fee_rate: Bounded::new(fee_rate).unwrap(),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        // Mock the oracle to return a price of 1 with 6 decimals for both assets.
        let mut oracle_querier = OracleQuerier::new_mock(oracle_prices);

        let (reserve, output) = pair
            .swap_exact_amount_in(
                &mut oracle_querier,
                &eth::DENOM.clone(),
                &usdc::DENOM.clone(),
                reserve,
                input,
            )
            .unwrap();

        assert_eq!(output, expected_output);
        assert_eq!(reserve, expected_reserve_after_swap);
    }

    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(usdc::DENOM.clone(), 4900500).unwrap(),
        Coin::new(eth::DENOM.clone(), 5000000).unwrap(),
        coin_pair! {
            usdc::DENOM.clone() => 10000000 - 4900500,
            eth::DENOM.clone() => 10000000 + 5000000,
        };
        "geometric pool 1:1 price swap out quote denom amount matches first order"
    )]
    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(eth::DENOM.clone(), 4900500).unwrap(),
        Coin::new(usdc::DENOM.clone(), 4999500).unwrap(),
        coin_pair! {
            eth::DENOM.clone() => 10000000 - 4900500,
            usdc::DENOM.clone() => 10000000 + 4999500,
        };
        "geometric pool 1:1 price swap out base denom amount matches first order"
    )]
    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(usdc::DENOM.clone(), 5050505 + 100000).unwrap(),
        Coin::new(eth::DENOM.clone(), 5463836).unwrap(),
        coin_pair! {
            usdc::DENOM.clone() => 10000000 - 5050505 - 100000,
            eth::DENOM.clone() => 10000000 + 5463836,
        };
        "geometric pool 1:1 price swap out quote denom amount matches first order part of second order"
    )]
    #[test_case(
        PassiveLiquidity::Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            order_spacing: Udec128::new_percent(50),
        },
        coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 10000000,
        },
        hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
            usdc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Udec128::new_percent(100),
                Timestamp::from_seconds(1730802926),
                6,
            ),
        },
        Udec128::new_percent(1),
        Coin::new(eth::DENOM.clone(), 5100000).unwrap(),
        Coin::new(usdc::DENOM.clone(), 5050000 + 228790).unwrap(),
        coin_pair! {
            eth::DENOM.clone() => 10000000 - 5100000,
            usdc::DENOM.clone() => 10000000 + 5050000 + 228790,
        };
        "geometric pool 1:1 price swap out base denom amount matches first order part of second order"
    )]
    fn swap_exact_amount_out(
        pool_type: PassiveLiquidity,
        reserve: CoinPair,
        oracle_prices: HashMap<Denom, PrecisionedPrice>,
        fee_rate: Udec128,
        output: Coin,
        expected_input: Coin,
        expected_reserve_after_swap: CoinPair,
    ) {
        let pair = PairParams {
            pool_type,
            swap_fee_rate: Bounded::new(fee_rate).unwrap(),
            lp_denom: Denom::new_unchecked(vec!["lp".to_string()]),
        };

        // Mock the oracle to return a price of 1 with 6 decimals for both assets.
        let mut oracle_querier = OracleQuerier::new_mock(oracle_prices);

        let (reserve, input) = pair
            .swap_exact_amount_out(
                &mut oracle_querier,
                &eth::DENOM.clone(),
                &usdc::DENOM.clone(),
                reserve,
                output,
            )
            .unwrap();

        assert_eq!(input, expected_input);
        assert_eq!(reserve, expected_reserve_after_swap);
    }
}
