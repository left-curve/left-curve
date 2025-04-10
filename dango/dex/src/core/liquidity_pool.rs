use {
    crate::TradingFunction,
    anyhow::ensure,
    dango_types::dex::{CurveInvariant, PairParams},
    grug::{
        Coin, CoinPair, Inner, IsZero, MultiplyFraction, MultiplyRatio, Number, NumberConst,
        Udec128, Uint128,
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

    /// Simulate a swap with an exact amount of input and a variable output.
    ///
    /// ## Inputs
    ///
    /// - `reserve`: The current pool reserves, before the swap is performed.
    /// - `input`: The amount of input asset to swap.
    ///
    /// ## Outputs
    ///
    /// - The amount of output asset received from the swap.
    fn simulate_swap_exact_amount_in(&self, reserve: CoinPair, input: Coin)
    -> anyhow::Result<Coin>;

    /// Simulate a swap with a variable amount of input and an exact output.
    ///
    /// ## Inputs
    ///
    /// - `reserve`: The current pool reserves, before the swap is performed.
    /// - `output`: The amount of output asset to receive.
    ///
    /// ## Outputs
    ///
    /// - The amount of input asset required to receive the exact output.
    fn simulate_swap_exact_amount_out(
        &self,
        reserve: CoinPair,
        output: Coin,
    ) -> anyhow::Result<Coin>;
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
        reserve: CoinPair,
        input: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)> {
        let mut new_reserve = reserve.clone();

        let output = self.simulate_swap_exact_amount_in(reserve, input.clone())?;

        new_reserve.checked_add(&input)?.checked_sub(&output)?;

        Ok((new_reserve, output))
    }

    fn swap_exact_amount_out(
        &self,
        reserve: CoinPair,
        output: Coin,
    ) -> anyhow::Result<(CoinPair, Coin)> {
        let mut new_reserve = reserve.clone();

        let input = self.simulate_swap_exact_amount_out(reserve, output.clone())?;

        new_reserve.checked_add(&input)?.checked_sub(&output)?;

        Ok((new_reserve, input))
    }

    fn simulate_swap_exact_amount_in(
        &self,
        reserve: CoinPair,
        input: Coin,
    ) -> anyhow::Result<Coin> {
        let output_denom = if reserve.first().denom == &input.denom {
            reserve.second().denom.clone()
        } else {
            reserve.first().denom.clone()
        };

        let a = reserve.amount_of(&input.denom)?;
        let b = reserve.amount_of(&output_denom)?;
        match self.curve_invariant {
            CurveInvariant::Xyk => {
                // Solve A * B = (A + offer.amount) * (B - amount_out) for amount_out
                // => amount_out = B - (A * B) / (A + offer.amount)
                // Round so that user takes the loss
                let amount_out = b.checked_sub(
                    Uint128::ONE.checked_multiply_ratio_ceil(a * b, a + input.amount)?,
                )?;

                // Apply swap fee. Round so that user takes the loss
                let amount_out = amount_out.checked_mul_dec_floor(
                    Udec128::ONE - self.swap_fee_rate.clone().into_inner(),
                )?;

                Ok(Coin {
                    denom: output_denom.clone(),
                    amount: amount_out,
                })
            },
        }
    }

    fn simulate_swap_exact_amount_out(
        &self,
        reserve: CoinPair,
        output: Coin,
    ) -> anyhow::Result<Coin> {
        let denom_in = if reserve.first().denom == &output.denom {
            reserve.second().denom.clone()
        } else {
            reserve.first().denom.clone()
        };

        let offer_reserves = reserve.amount_of(&denom_in)?;
        let ask_reserves = reserve.amount_of(&output.denom)?;
        ensure!(offer_reserves > output.amount, "insufficient liquidity");

        match self.curve_invariant {
            CurveInvariant::Xyk => {
                // Apply swap fee. In SwapExactIn we multiply ask by (1 - fee) to get the
                // offer amount after fees. So in this case we need to divide ask by (1 - fee)
                // to get the ask amount after fees. Round so that user takes the loss
                let coin_out_after_fee = output
                    .amount
                    .checked_div_dec_ceil(Udec128::ONE - self.swap_fee_rate.clone().into_inner())?;

                // Solve A * B = (A + amount_in) * (B - ask.amount) for amount_in
                // => amount_in = (A * B) / (B - ask.amount) - A
                // Round so that user takes the loss
                let amount_in = Uint128::ONE
                    .checked_multiply_ratio_floor(
                        offer_reserves * ask_reserves,
                        ask_reserves - coin_out_after_fee,
                    )?
                    .checked_sub(offer_reserves)?;

                Ok(Coin {
                    denom: denom_in.clone(),
                    amount: amount_in,
                })
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
