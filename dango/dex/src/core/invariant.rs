use {
    crate::{secant_method, solidly_log_invariant},
    anyhow::ensure,
    dango_types::dex::CurveInvariant,
    grug::{
        Coin, CoinPair, Dec256, Decimal, Denom, Int, MultiplyFraction, MultiplyRatio, NextNumber,
        Number, NumberConst, PrevNumber, Signed, Udec128, Uint128, Uint256, Unsigned,
    },
};

pub trait TradingFunction {
    /// Calculate the value of the trading invariant.
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint256>;

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
    fn invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint256> {
        match self {
            // k = x * y
            CurveInvariant::Xyk => {
                let first = reserve.first().amount.into_next();
                let second = reserve.second().amount.into_next();
                Ok(first.checked_mul(second)?)
            },
            CurveInvariant::Solidly => {
                let first = reserve.first().amount.clone();
                let second = reserve.second().amount.clone();

                Ok(solidly_log_invariant(
                    first
                        .into_next()
                        .checked_into_dec()?
                        .checked_into_signed()?,
                    second
                        .into_next()
                        .checked_into_dec()?
                        .checked_into_signed()?,
                )?
                .checked_floor()?
                .checked_into_unsigned()?
                .into_int())
            },
        }
    }

    fn normalized_invariant(&self, reserve: &CoinPair) -> anyhow::Result<Uint128> {
        match self {
            // sqrt(k)
            CurveInvariant::Xyk => Ok(self
                .invariant(reserve)?
                .checked_sqrt()?
                .checked_into_prev()?),
            // sqrt(sqrt(k))
            CurveInvariant::Solidly => Ok(self
                .invariant(reserve)?
                .checked_sqrt()?
                .checked_sqrt()?
                .checked_into_prev()?),
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

        let amount_in = match *self {
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

                amount_in
            },
            CurveInvariant::Solidly => {
                let offer_reserves_dec256: Dec256 = offer_reserves
                    .into_next()
                    .checked_into_dec()?
                    .checked_into_signed()?;
                let amount_in = solve_amount(
                    solidly_log_invariant,
                    (offer_reserves, ask_reserves),
                    ask_reserves.checked_sub(coin_out.amount)?,
                    |amount| Ok(offer_reserves_dec256.checked_add(amount)?),
                    coin_out
                        .amount
                        .into_next()
                        .checked_into_dec()?
                        .checked_into_signed()?,
                    Dec256::new_bps(1),
                )?;

                amount_in
                // let amount_in = Int::ONE.checked_multiply_ratio_ceil(
                //     offer_reserves * ask_reserves,
                //     ask_reserves - coin_out_after_fee,
                // )? - offer_reserves;
            },
        };

        Ok(Coin {
            denom: denom_in.clone(),
            amount: amount_in,
        })
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

        let a = reserves.amount_of(&coin_in.denom)?;
        let b = reserves.amount_of(denom_out)?;

        let amount_out = match self {
            CurveInvariant::Xyk => {
                // Solve A * B = (A + offer.amount) * (B - amount_out) for amount_out
                // => amount_out = B - (A * B) / (A + offer.amount)
                // Round so that user takes the loss
                let amount_out = b - Int::ONE.checked_multiply_ratio_ceil(
                    a.checked_mul(b)?,
                    a.checked_add(coin_in.amount)?,
                )?;

                // Apply swap fee. Round so that user takes the loss
                let amount_out = amount_out.checked_mul_dec_floor(Udec128::ONE - swap_fee)?;

                amount_out
            },
            CurveInvariant::Solidly => {
                let b_dec256: Dec256 = b.into_next().checked_into_dec()?.checked_into_signed()?;
                let amount_out = solve_amount(
                    solidly_log_invariant,
                    (a, b),
                    a.checked_add(coin_in.amount)?,
                    |amount| Ok(b_dec256.checked_sub(amount)?),
                    coin_in
                        .amount
                        .into_next()
                        .checked_into_dec()?
                        .checked_into_signed()?,
                    Dec256::new_bps(1),
                )?;

                amount_out
            },
        };

        Ok(Coin {
            denom: denom_out.clone(),
            amount: amount_out,
        })
    }
}

fn solve_amount<F, G>(
    invariant: F,
    reserves_before: (Uint128, Uint128),
    a_after: Uint128,
    calc_b_after: G,
    initial_guess: Dec256,
    tolerance: Dec256,
) -> anyhow::Result<Uint128>
where
    F: Fn(Dec256, Dec256) -> anyhow::Result<Dec256>,
    G: Fn(Dec256) -> anyhow::Result<Dec256>,
{
    let a = reserves_before
        .0
        .into_next()
        .checked_into_dec()?
        .checked_into_signed()?;
    let b = reserves_before
        .1
        .into_next()
        .checked_into_dec()?
        .checked_into_signed()?;

    let invariant_before = invariant(a, b)?;

    let a_after: Dec256 = a_after
        .into_next()
        .checked_into_dec()?
        .checked_into_signed()?;

    // Set up the function to solve for amount out using secant method which is
    // invariant(a_after, b_after) - invariant(a, b) = 0
    let g = |amount: Dec256| -> anyhow::Result<Dec256> {
        let b_after = calc_b_after(amount)?;
        let invariant_after = invariant(a_after, b_after)?;

        Ok(invariant_after.checked_sub(invariant_before)?)
    };

    let amount_out = secant_method(
        g,
        initial_guess,
        initial_guess.checked_sub(Dec256::ONE)?,
        tolerance,
        100,
    )?;

    println!("amount_out before floor: {}", amount_out);

    Ok(amount_out
        .checked_floor()?
        .into_int()
        .checked_into_unsigned()?
        .checked_into_prev()?)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use test_case::test_case;

    use crate::solidly_log_invariant;

    use super::*;

    #[test_case(
        100_000u128,  // reserve_a
        100_000u128,  // reserve_b
        50_000u128,   // amount_in
        47_260u128    // expected_amount_out
        ; "balanced reserves with 50% swap"
    )]
    #[test_case(
        100_000_000_000_000_000u128,  // reserve_a
        100_000_000_000_000_000u128,  // reserve_b
        50_000_000_000_000_000u128,   // amount_in
        47_260_433_708_134_015u128    // expected_amount_out
        ; "balanced massive reserves with 50% swap"
    )]
    #[test_case(
        1_000_000_000_000_000_000_000_000u128,  // reserve_a
        1_000_000_000_000_000_000_000_000u128,  // reserve_b
        1_000_000_000_000_000_000_000u128,   // amount_in
        900_000_000_000_000_000_000u128    // expected_amount_out
        ; "massive reserves with 10% swap"
    )]
    fn test_solidly_curve_solve_amount_out(
        reserve_a: u128,
        reserve_b: u128,
        amount_in: u128,
        expected_amount_out: u128,
    ) {
        let amount_out = CurveInvariant::Solidly
            .solve_amount_out(
                Coin::new(Denom::from_str("uatom").unwrap(), amount_in).unwrap(),
                &Denom::from_str("uosmo").unwrap(),
                Udec128::ZERO,
                &CoinPair::new(
                    Coin::new(Denom::from_str("uatom").unwrap(), reserve_a).unwrap(),
                    Coin::new(Denom::from_str("uosmo").unwrap(), reserve_b).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(amount_out.amount, Uint128::from(expected_amount_out));
    }

    #[test_case(
        100_000u128,  // reserve_a
        100_000u128,  // reserve_b
        47_260u128,   // amount_out
        50_000u128    // expected_amount_in
        ; "balanced reserves with 50% swap"
    )]
    fn test_xyk_curve_solve_amount_in(
        reserve_a: u128,
        reserve_b: u128,
        amount_out: u128,
        expected_amount_in: u128,
    ) {
        let amount_in = CurveInvariant::Xyk
            .solve_amount_in(
                Coin::new(Denom::from_str("uosmo").unwrap(), amount_out).unwrap(),
                &Denom::from_str("uatom").unwrap(),
                Udec128::ZERO,
                &CoinPair::new(
                    Coin::new(Denom::from_str("uatom").unwrap(), reserve_a).unwrap(),
                    Coin::new(Denom::from_str("uosmo").unwrap(), reserve_b).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(amount_in.amount, Uint128::from(expected_amount_in));
    }
}
