use {
    crate::TradingFunction,
    anyhow::ensure,
    dango_types::dex::{Direction, PairParams, SlippageControl},
    grug::{
        Coin, CoinPair, Denom, Inner, IsZero, MultiplyFraction, Number, NumberConst, Udec128,
        Uint128,
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
    /// ## Inputs:
    ///
    /// - `reserve`: The current pool reserves, before the withdrawal is made.
    /// - `lp_token_supply`: The current total supply of LP tokens.
    /// - `lp_burn_amount`: The amount of LP tokens to burn.
    ///
    /// ## Outputs:
    ///
    /// - The updated pool reserves.
    /// - The funds withdrawn from the pool.
    fn remove_liquidity(
        &self,
        reserve: CoinPair,
        lp_token_supply: Uint128,
        lp_burn_amount: Uint128,
    ) -> anyhow::Result<(CoinPair, CoinPair)>;

    /// Perform a swap in the pool. This function mutates the pool reserves.
    ///
    /// ## Inputs
    ///
    /// - `swap`: The swap request containing the information of the swap to be performed.
    ///
    /// ## Outputs
    ///
    /// - The updated pool reserves.
    /// - The coin input to the swap.
    /// - The coin output from the swap.
    fn swap(
        &self,
        reserve: CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        slippage: Option<SlippageControl>,
    ) -> anyhow::Result<(CoinPair, Coin, Coin)>;

    /// Simulate a swap in the pool. This function does not mutate the pool reserves.
    ///
    /// ## Inputs
    ///
    /// - `swap`: The swap request containing the information of the swap to be performed.
    ///
    /// ## Outputs
    ///
    /// - A tuple of coins, where the first coin is the offer and the second coin is the ask.
    fn simulate_swap(
        &self,
        reserve: &CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        slippage: Option<SlippageControl>,
    ) -> anyhow::Result<(Coin, Coin)>;
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

    fn swap(
        &self,
        mut reserve: CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        slippage: Option<SlippageControl>,
    ) -> anyhow::Result<(CoinPair, Coin, Coin)> {
        let invariant_before = self.curve_invariant.invariant(&reserve)?;

        let (coin_in, coin_out) = self.simulate_swap(
            &reserve,
            base_denom,
            quote_denom,
            direction,
            amount,
            slippage,
        )?;

        reserve.checked_add(&coin_in)?.checked_sub(&coin_out)?;

        // Sanity check that the invariant is preserved. Should be larger
        // after in all swaps with fee.
        let invariant_after = self.curve_invariant.invariant(&reserve)?;
        ensure!(
            invariant_after >= invariant_before,
            "invariant not preserved"
        );

        Ok((reserve, coin_in, coin_out))
    }

    fn simulate_swap(
        &self,
        reserves: &CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        slippage: Option<SlippageControl>,
    ) -> anyhow::Result<(Coin, Coin)> {
        ensure!(
            reserves.has(&base_denom) && reserves.has(&quote_denom),
            "invalid reserves"
        );

        // Perform the swap calculations
        let (coin_in, coin_out) = match direction {
            Direction::Ask => {
                let coin_in = Coin::new(base_denom.clone(), amount)?;
                (
                    coin_in.clone(),
                    self.curve_invariant.solve_amount_out(
                        coin_in.clone(),
                        &quote_denom,
                        self.swap_fee_rate.clone().into_inner(),
                        reserves,
                    )?,
                )
            },
            Direction::Bid => {
                let coin_out = Coin::new(base_denom.clone(), amount)?;
                (
                    self.curve_invariant.solve_amount_in(
                        coin_out.clone(),
                        &quote_denom,
                        self.swap_fee_rate.clone().into_inner(),
                        reserves,
                    )?,
                    coin_out,
                )
            },
        };

        // Enforce slippage control.
        if let Some(slippage_control) = slippage {
            match slippage_control {
                SlippageControl::MinimumOut(min_out) => {
                    ensure!(
                        direction != Direction::Bid,
                        "minimum out is only supported for direction: ask"
                    );
                    ensure!(coin_out.amount >= min_out, "slippage tolerance exceeded");
                },
                SlippageControl::MaximumIn(max_in) => {
                    ensure!(
                        direction != Direction::Ask,
                        "maximum in is only supported for direction: bid"
                    );
                    ensure!(coin_in.amount <= max_in, "slippage tolerance exceeded");
                },
                SlippageControl::PriceLimit(price_limit) => {
                    let execution_price =
                        Udec128::checked_from_ratio(coin_out.amount, coin_in.amount)?;
                    match direction {
                        Direction::Bid => ensure!(
                            execution_price <= price_limit,
                            "slippage tolerance exceeded"
                        ),
                        Direction::Ask => ensure!(
                            execution_price >= price_limit,
                            "slippage tolerance exceeded"
                        ),
                    }
                },
            }
        }

        Ok((coin_in, coin_out))
    }
}

fn abs_diff(a: Uint128, b: Uint128) -> Uint128 {
    if a > b {
        a - b
    } else {
        b - a
    }
}
