use {
    crate::{PoolExt, PoolInit},
    anyhow::bail,
    dango_types::amm::{XykParams, XykPool},
    grug::{
        Coin, CoinPair, Inner, MultiplyFraction, MultiplyRatio, NextNumber, Number, PrevNumber,
        Uint128,
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
        // output = ask_pool - (ask_pool * offer_pool) / (offer_pool + input)
        let mut output = ask
            .amount
            .checked_sub(ask.amount.checked_multiply_ratio_floor(
                *offer.amount,
                offer.amount.checked_add(input.amount)?,
            )?)?;

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
