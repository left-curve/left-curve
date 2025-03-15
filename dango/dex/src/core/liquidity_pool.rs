use {
    crate::TradingFunction,
    anyhow::ensure,
    dango_types::dex::{CreateLimitOrderRequest, CurveInvariant, Direction, PairParams},
    grug::{
        Coin, CoinPair, Denom, Inner, IsZero, MultiplyFraction, Number, NumberConst, StdResult,
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
    ) -> anyhow::Result<(CoinPair, Coin, Coin)>;

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
    /// - A vec of orders to place on the orderbook.
    fn reflect_curve(
        &self,
        base_denom: Denom,
        quote_denom: Denom,
        reserves: &CoinPair,
    ) -> StdResult<Vec<CreateLimitOrderRequest>>;

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

    fn swap(
        &self,
        reserve: CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
    ) -> anyhow::Result<(CoinPair, Coin, Coin)> {
        let invariant_before = self.curve_invariant.invariant(&reserve)?;

        let (new_reserve, coin_in, coin_out) =
            self.simulate_swap(&reserve, base_denom, quote_denom, direction, amount)?;

        // Sanity check that the invariant is preserved. Should be larger
        // after in all swaps with fee.
        let invariant_after = self.curve_invariant.invariant(&reserve)?;
        ensure!(
            invariant_after >= invariant_before,
            "invariant not preserved"
        );

        Ok((new_reserve, coin_in, coin_out))
    }

    fn simulate_swap(
        &self,
        reserve: &CoinPair,
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
    ) -> anyhow::Result<(CoinPair, Coin, Coin)> {
        ensure!(
            reserve.has(&base_denom) && reserve.has(&quote_denom),
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
                        reserve,
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
                        reserve,
                    )?,
                    coin_out,
                )
            },
        };

        let mut new_reserve = reserve.clone();
        new_reserve.checked_add(&coin_in)?.checked_sub(&coin_out)?;

        Ok((new_reserve, coin_in, coin_out))
    }

    fn reflect_curve(
        &self,
        base_denom: Denom,
        quote_denom: Denom,
        reserves: &CoinPair,
    ) -> StdResult<Vec<CreateLimitOrderRequest>> {
        let a = reserves.amount_of(&base_denom)?;
        let b = reserves.amount_of(&quote_denom)?;

        let price = self.spot_price(&base_denom, &quote_denom, reserves)?;

        // Calculate the starting price for the ask and bid side respectively. We
        // place the spread symmetrically around the spot price.
        let swap_fee_rate = self.swap_fee_rate.clone().into_inner();
        let starting_price_ask = price.checked_mul(Udec128::ONE.checked_add(swap_fee_rate)?)?;
        let starting_price_bid = price.checked_mul(Udec128::ONE.checked_sub(swap_fee_rate)?)?;

        let mut orders = Vec::with_capacity(2 * self.order_depth as usize);
        let mut a_bid_prev = Uint128::ZERO;
        let mut a_ask_prev = Uint128::ZERO;
        for i in 0..(self.order_depth + 1) {
            let delta_p = self
                .tick_size
                .checked_mul(Udec128::checked_from_ratio(i as u128, Uint128::ONE)?)?;

            // Calculate the price i ticks on the ask and bid side respectively
            let price_ask = starting_price_ask.checked_add(delta_p)?;
            let price_bid = starting_price_bid.checked_sub(delta_p)?;

            // Calculate the amount of base that can be bought at the price
            let (a_ask, a_bid) = match self.curve_invariant {
                CurveInvariant::Xyk => {
                    let a_ask = a.checked_sub(b.checked_div_dec(price_ask)?)?;
                    let a_bid = b.checked_div_dec(price_bid)?.checked_sub(a)?;
                    (a_ask, a_bid)
                },
            };

            orders.push(CreateLimitOrderRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
                direction: Direction::Bid,
                amount: a_bid.checked_sub(a_bid_prev)?,
                price: price_bid,
            });

            orders.push(CreateLimitOrderRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
                direction: Direction::Ask,
                amount: a_ask.checked_sub(a_ask_prev)?,
                price: price_ask,
            });

            a_bid_prev = a_bid;
            a_ask_prev = a_ask;
        }

        Ok(orders)
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
