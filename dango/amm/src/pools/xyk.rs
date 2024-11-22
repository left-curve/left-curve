use {
    crate::{PoolExt, PoolInit},
    anyhow::bail,
    dango_types::amm::{XykParams, XykPool},
    grug::{
        Coin, CoinPair, Inner, MultiplyFraction, MultiplyRatio, NextNumber, Number, NumberConst,
        PrevNumber, Udec128, Uint128,
    },
};

impl PoolInit for XykPool {
    type Params = XykParams;

    fn initialize(liquidity: CoinPair, params: XykParams) -> anyhow::Result<Self> {
        let shares = liquidity
            .first()
            .amount
            .checked_full_mul(*liquidity.second().amount)?
            .checked_sqrt()?
            .checked_into_prev()?;

        Ok(Self {
            params,
            liquidity,
            shares,
        })
    }
}

impl PoolExt for XykPool {
    fn swap(&mut self, input: Coin) -> anyhow::Result<(Coin, Coin)> {
        let (offer, ask) = if input.denom == *self.liquidity.first().denom {
            self.liquidity.as_mut()
        } else if input.denom == *self.liquidity.second().denom {
            self.liquidity.as_mut_rev()
        } else {
            bail!(
                "invalid input denom! must be {}|{}, got: {}",
                self.liquidity.first().denom,
                self.liquidity.second().denom,
                input.denom
            );
        };

        // Compute swap output.
        //
        // ask_pool * offer_pool = (ask_pool - output) * (offer_pool + input)
        // output = ask_pool * input / (offer_pool + input)
        let mut output = ask
            .amount
            .checked_multiply_ratio_floor(input.amount, offer.amount.checked_add(input.amount)?)?;

        // Compute liquidity fee. (Note: use ceil rounding.)
        let liquidity_fee = output.checked_mul_dec_ceil(*self.params.liquidity_fee_rate.inner())?;

        // Deduct liquidity fee from the output.
        output.checked_sub_assign(liquidity_fee)?;

        // Update pool state.
        offer.amount.checked_add_assign(input.amount)?;
        ask.amount.checked_sub_assign(output)?;

        Ok((
            Coin {
                denom: ask.denom.clone(),
                amount: output,
            },
            Coin {
                denom: ask.denom.clone(),
                amount: liquidity_fee,
            },
        ))
    }

    fn reverse_swap(&self, mut output: Coin) -> anyhow::Result<(Coin, Coin)> {
        let (ask, offer) = if output.denom == *self.liquidity.first().denom {
            self.liquidity.as_ref()
        } else if output.denom == *self.liquidity.second().denom {
            self.liquidity.as_ref_rev()
        } else {
            bail!(
                "invalid input denom! must be {}|{}, got: {}",
                self.liquidity.first().denom,
                self.liquidity.second().denom,
                output.denom
            );
        };

        // Liquidity_fee = output / (1 - fee_rate) - output
        let liquidity_fee = output
            .amount
            .checked_div_dec_ceil(Udec128::ONE - *self.params.liquidity_fee_rate.inner())?
            - output.amount;

        // Add liquidity fee to the output
        output.amount += liquidity_fee;

        // Compute swap input requested for the given output.
        //
        // ask_pool * offer_pool = (ask_pool - output) * (offer_pool + input)
        // input = offer_pool * output / (ask_pool - output)
        let input = offer
            .amount
            .checked_multiply_ratio_ceil(output.amount, ask.amount.checked_sub(output.amount)?)?;

        Ok((
            Coin {
                denom: offer.denom.clone(),
                amount: input,
            },
            Coin {
                denom: ask.denom.clone(),
                amount: liquidity_fee,
            },
        ))
    }

    // See `liquidity-providion.md` in docs for the math used here.
    fn provide_liquidity(&mut self, deposit: CoinPair) -> anyhow::Result<Uint128> {
        let pool1 = self.liquidity.first().amount.into_next();
        let pool2 = self.liquidity.second().amount.into_next();

        let user1 = deposit.first().amount.into_next();
        let user2 = deposit.second().amount.into_next();

        let shares_before = self.shares;
        let shares_after = shares_before
            .into_next()
            .checked_pow(2)?
            .checked_mul(pool1.checked_add(user1)?)?
            .checked_mul(pool2.checked_add(user2)?)?
            .checked_div(pool1)?
            .checked_div(pool2)?
            .checked_sqrt()?
            .checked_into_prev()?;

        self.shares = shares_after;
        self.liquidity.merge(deposit)?;

        Ok(shares_after - shares_before)
    }

    fn withdraw_liquidity(&mut self, shares_to_burn: Uint128) -> anyhow::Result<CoinPair> {
        let shares_before = self.shares;

        self.shares = shares_before.checked_sub(shares_to_burn)?;

        Ok(self.liquidity.split(shares_to_burn, shares_before)?)
    }
}

#[cfg(test)]
mod test {
    use {
        crate::PoolExt,
        dango_types::amm::{FeeRate, XykParams, XykPool},
        grug::{Coin, CoinPair, NumberConst, Udec128, Uint128},
    };

    #[test]
    fn reverse_swap() {
        let pool = XykPool {
            params: XykParams {
                liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_percent(1)),
            },
            liquidity: CoinPair::new_unchecked(
                Coin::new("usdc", 100_000).unwrap(),
                Coin::new("eth", 50_000).unwrap(),
            ),
            shares: Uint128::ZERO,
        };

        // use reverse_input cause approximations
        for (input, output_request, fee_request, reverse_input) in [
            (1_000, 495, 5, 1_000),
            (10_000, 4_545, 46, 9_999),
            (100_000, 25_000, 250, 100_000),
        ] {
            let (output, fee) = pool
                .clone()
                .swap(Coin::new("usdc", input).unwrap())
                .unwrap();

            assert_eq!(
                output,
                Coin::new("eth", output_request - fee_request).unwrap()
            );
            assert_eq!(fee, Coin::new("eth", fee_request).unwrap());

            let (calculated_input, fee) = pool.clone().reverse_swap(output.clone()).unwrap();

            assert_eq!(calculated_input, Coin::new("usdc", reverse_input).unwrap());
            assert_eq!(fee, Coin::new("eth", fee_request).unwrap());
        }
    }
}
