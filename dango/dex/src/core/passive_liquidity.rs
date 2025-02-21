use {
    anyhow::ensure,
    dango_types::dex::{CurveInvariant, Direction, Pool, Swap},
    grug::{
        Coin, CoinPair, Denom, Int, IsZero, MultiplyFraction, MultiplyRatio, Number, NumberConst,
        Udec128, Uint128,
    },
};

pub trait PassiveLiquidityPool {
    /// Initialize the pool with the given reserves.
    ///
    /// Returns a tuple of the pool and the initial LP token supply.
    fn initialize(
        base_denom: Denom,
        quote_denom: Denom,
        reserves: CoinPair,
        curve_type: CurveInvariant,
        swap_fee: Udec128,
    ) -> anyhow::Result<(Box<Self>, Uint128)>;

    /// Provide liquidity to the pool. This function mutates the pool reserves.
    /// Liquidity is provided at the current pool balance, any excess funds are
    /// returned to the user.
    ///
    /// Returns a tuple of (`Udec128`, `CoinPair`) where the first element is the
    /// percentage by which to increase the LP token supply and the second element
    /// is the amount of each asset that was not used to provide liquidity.
    fn add_liquidity(&mut self, funds: CoinPair) -> anyhow::Result<Udec128>;

    /// Remove a portion of the liquidity from the pool. This function mutates the pool reserves.
    ///
    /// Returns the underlying liquidity that was removed.
    fn remove_liquidity(
        &mut self,
        numerator: Uint128,
        denominator: Uint128,
    ) -> anyhow::Result<CoinPair>;

    /// Perform a swap in the pool. This function mutates the pool reserves.
    ///
    /// Returns a tuple of coins, where the first coin is the offer and the second
    /// coin is the ask.
    fn swap(&mut self, swap: &Swap) -> anyhow::Result<(Coin, Coin)>;

    /// Simulate a swap in the pool. This function does not mutate the pool reserves.
    ///
    /// Returns a tuple of coins, where the first coin is the offer and the second
    /// coin is the ask.
    fn simulate_swap(&self, swap: &Swap) -> anyhow::Result<(Coin, Coin)>;
}

pub trait TradingFunction {
    /// Calculate the value of the trading invariant.
    fn invariant(&self, reserves: &CoinPair) -> anyhow::Result<Uint128>;

    /// Solve the trading function for the amount of output coins given an
    /// amount of input coins.
    fn solve_amount_out(
        &self,
        coin_in: Coin,
        denom_out: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin>;

    /// Solve the trading function for the amount of input coins given an
    /// amount of output coin.
    fn solve_amount_in(
        &self,
        coin_out: Coin,
        denom_in: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin>;
}

impl PassiveLiquidityPool for Pool {
    fn initialize(
        base_denom: Denom,
        quote_denom: Denom,
        reserves: CoinPair,
        curve_type: CurveInvariant,
        swap_fee: Udec128,
    ) -> anyhow::Result<(Box<Self>, Uint128)> {
        ensure!(
            reserves.first().amount.is_non_zero() && reserves.second().amount.is_non_zero(),
            "cannot initialize pool with zero reserves"
        );

        ensure!(
            reserves.has(&base_denom) && reserves.has(&quote_denom),
            "invalid reserves"
        );

        let initial_lp_supply = match curve_type {
            CurveInvariant::Xyk => curve_type.invariant(&reserves)?.checked_sqrt()?,
        };

        Ok((
            Box::new(Self {
                base_denom,
                quote_denom,
                reserves,
                curve_type,
                swap_fee,
            }),
            initial_lp_supply,
        ))
    }

    fn add_liquidity(&mut self, funds: CoinPair) -> anyhow::Result<Udec128> {
        ensure!(
            self.reserves.first().amount.is_non_zero()
                && self.reserves.second().amount.is_non_zero(),
            "cannot add liquidity to pool with zero reserves"
        );

        ensure!(
            funds.first().denom == self.reserves.first().denom
                && funds.second().denom == self.reserves.second().denom,
            "invalid funds"
        );

        ensure!(
            funds.first().amount.is_non_zero() && funds.second().amount.is_non_zero(),
            "cannot add zero liquidity"
        );

        let invariant_before = self.curve_type.invariant(&self.reserves)?;

        // Add the used funds to the pool reserves
        self.reserves
            .checked_add(&Coin::new(
                funds.first().denom.clone(),
                *funds.first().amount,
            )?)?
            .checked_add(&Coin::new(
                funds.second().denom.clone(),
                *funds.second().amount,
            )?)?;

        // Compute the proportional increase in the invariant
        let invariant_after = self.curve_type.invariant(&self.reserves)?;
        let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

        // Compute the mint ratio from the invariant ratio based on the curve type.
        // This ensures that an unbalances provision will be equivalent to a swap
        // followed by a balancedliquidity provision.
        let mint_ratio = match self.curve_type {
            CurveInvariant::Xyk => invariant_ratio.checked_sqrt()?,
        }
        .checked_sub(Udec128::ONE)?;

        Ok(mint_ratio)
    }

    fn remove_liquidity(
        &mut self,
        numerator: Uint128,
        denominator: Uint128,
    ) -> anyhow::Result<CoinPair> {
        Ok(self.reserves.split(numerator, denominator)?)
    }

    fn swap(&mut self, swap: &Swap) -> anyhow::Result<(Coin, Coin)> {
        let invariant_before = self.curve_type.invariant(&self.reserves)?;

        let (coin_in, coin_out) = self.simulate_swap(swap)?;

        self.reserves
            .checked_add(&coin_in)?
            .checked_sub(&coin_out)?;

        // Sanity check that the invariant is preserved. Should be larger
        // after in all swaps with fee.
        let invariant_after = self.curve_type.invariant(&self.reserves)?;
        ensure!(
            invariant_after >= invariant_before,
            "invariant not preserved"
        );

        Ok((coin_in, coin_out))
    }

    fn simulate_swap(&self, swap: &Swap) -> anyhow::Result<(Coin, Coin)> {
        ensure!(
            self.reserves.has(&swap.base_denom) && self.reserves.has(&swap.quote_denom),
            "invalid reserves"
        );

        let (coin_in, coin_out) = match swap.direction {
            Direction::Ask => {
                let coin_in = Coin::new(swap.base_denom.clone(), swap.amount)?;
                (
                    coin_in.clone(),
                    self.curve_type.solve_amount_out(
                        coin_in.clone(),
                        &swap.quote_denom,
                        self.swap_fee,
                        &self.reserves,
                    )?,
                )
            },
            Direction::Bid => {
                let coin_out = Coin::new(swap.base_denom.clone(), swap.amount)?;
                (
                    self.curve_type.solve_amount_in(
                        coin_out.clone(),
                        &swap.quote_denom,
                        self.swap_fee,
                        &self.reserves,
                    )?,
                    coin_out,
                )
            },
        };

        Ok((coin_in, coin_out))
    }
}

impl TradingFunction for CurveInvariant {
    fn invariant(&self, reserves: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            CurveInvariant::Xyk => Ok(*reserves.first().amount * *reserves.second().amount),
        }
    }

    fn solve_amount_out(
        &self,
        coin_in: Coin,
        denom_out: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin> {
        ensure!(
            reserves.has(&coin_in.denom) && reserves.has(denom_out),
            "invalid reserves"
        );
        match self {
            CurveInvariant::Xyk => {
                let a = reserves.first().amount.clone();
                let b = reserves.second().amount.clone();

                // Solve A * B = (A + offer.amount) * (B - amount_out) for amount_out
                // => amount_out = B - (A * B) / (A + offer.amount)
                // Round so that user takes the loss
                let amount_out =
                    b - Int::ONE.checked_multiply_ratio_ceil(a * b, a + coin_in.amount)?;

                // Apply swap fee. Round so that user takes the loss
                let amount_out = amount_out.checked_mul_dec_floor(Udec128::ONE - swap_fee)?;

                Ok(Coin {
                    denom: denom_out.clone(),
                    amount: amount_out,
                })
            },
        }
    }

    fn solve_amount_in(
        &self,
        coin_out: Coin,
        denom_in: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin> {
        ensure!(
            reserves.has(denom_in) && reserves.has(&coin_out.denom),
            "invalid reserves"
        );

        let offer_reserves = reserves.amount_of(denom_in);
        let ask_reserves = reserves.amount_of(&coin_out.denom);
        ensure!(offer_reserves > coin_out.amount, "insufficient liquidity");

        match *self {
            CurveInvariant::Xyk => {
                // Apply swap fee. In SwapExactIn we multiply ask by (1 - fee) to get the
                // offer amount after fees. So in this case we need to divide ask by (1 - fee)
                // to get the ask amount after fees. Round so that user takes the loss
                let coin_out_after_fee = coin_out
                    .amount
                    .checked_div_dec_ceil(Udec128::ONE - swap_fee)?;

                // Solve A * B = (A + amount_in) * (B - ask.amount) for amount_in
                // => amount_in = (A * B) / (B - ask.amount) - A
                // Round so that user takes the loss
                let amount_in = Int::ONE.checked_multiply_ratio_ceil(
                    offer_reserves * ask_reserves,
                    ask_reserves - coin_out_after_fee,
                )? - offer_reserves;

                Ok(Coin {
                    denom: denom_in.clone(),
                    amount: amount_in,
                })
            },
        }
    }
}
