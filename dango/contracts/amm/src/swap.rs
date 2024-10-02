use {
    crate::PoolExt,
    dango_types::amm::{Config, Pool, SwapOutcome},
    grug::{Coin, Coins, Inner, MultiplyFraction, Number},
};

// Note: this function assumes the swap route doesn't contain any loop, meaning
// the same pool must not appear twice in the `pools` iterator.
// The caller should make sure of this by using a `UniqueVec` when taking in
// the swap route.
pub fn perform_swap<'a, I>(cfg: &Config, mut input: Coin, pools: I) -> anyhow::Result<SwapOutcome>
where
    I: Iterator<Item = &'a mut Pool>,
{
    let mut liquidity_fees = Coins::new();

    // Iterate through the pools and perform swaps.
    for pool in pools {
        let (output, liquidity_fee) = match pool {
            Pool::Xyk(xyk) => xyk.swap(input)?,
            Pool::Concentrated(concentrated) => concentrated.swap(input)?,
        };

        // The output of this pool is the input for the next pool.
        input = output;

        // Track the liquidity fees charged.
        liquidity_fees.insert(liquidity_fee)?;
    }

    // This is the final swap output.
    let mut output = input;

    // Compute protocol fee. (Note: use ceil rounding.)
    let protocol_fee = output
        .amount
        .checked_mul_dec_ceil(*cfg.protocol_fee_rate.inner())?;

    // Deduct protocol fee from the output.
    output.amount = output.amount.checked_sub(protocol_fee)?;

    Ok(SwapOutcome {
        protocol_fee: Coin {
            denom: output.denom.clone(),
            amount: protocol_fee,
        },
        output,
        liquidity_fees,
    })
}
