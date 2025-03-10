use {
    anyhow::ensure,
    dango_types::dex::CurveInvariant,
    grug::{
        Coin, CoinPair, Denom, Int, MultiplyFraction, MultiplyRatio, Number, NumberConst, Udec128,
        Uint128,
    },
};

pub trait TradingFunction {
    /// Calculate the value of the trading invariant.
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128>;

    fn normalized_invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128>;

    fn solve_amount_in(
        &self,
        coin_out: Coin,
        denom_in: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin>;

    fn solve_amount_out(
        &self,
        coin_in: Coin,
        denom_out: &Denom,
        swap_fee: Udec128,
        reserves: &CoinPair,
    ) -> anyhow::Result<Coin>;
}

impl TradingFunction for CurveInvariant {
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            // k = x * y
            CurveInvariant::Xyk => {
                let first = *reserve.first().amount;
                let second = *reserve.second().amount;
                Ok(first.checked_mul(second)?)
            },
        }
    }

    fn normalized_invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            // sqrt(k)
            CurveInvariant::Xyk => Ok(self.invariant(reserve)?.checked_sqrt()?),
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

        let offer_reserves = reserves.amount_of(denom_in)?;
        let ask_reserves = reserves.amount_of(&coin_out.denom)?;
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
                let a = reserves.amount_of(&coin_in.denom)?;
                let b = reserves.amount_of(denom_out)?;

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
}
