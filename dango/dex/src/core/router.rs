use {
    crate::{PAIRS, PassiveLiquidityPool, RESERVES},
    dango_oracle::OracleQuerier,
    dango_types::dex::PairId,
    grug::{
        Coin, CoinPair, Inner, MultiplyFraction, NonZero, Number, Storage, Udec128, Uint128,
        UniqueVec,
    },
    std::collections::HashMap,
};

/// ## Returns
///
/// - The updated pool reserves of every pair visited in the route.
/// - The output after deducting the protocol fee.
/// - The protocol fee deducted.
pub fn swap_exact_amount_in(
    storage: &dyn Storage,
    oracle_querier: &mut OracleQuerier<'_>,
    protocol_fee_rate: Udec128,
    route: UniqueVec<PairId>,
    input: Coin,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin, Uint128)> {
    let mut reserves = HashMap::new();
    let mut output = input;

    for pair in route.into_iter() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pool's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Perform the swap.
        // The output of the previous step is the input of this step.
        (reserve, output) = params.swap_exact_amount_in(
            oracle_querier,
            &pair.base_denom,
            &pair.quote_denom,
            reserve,
            output,
        )?;

        // Save the updated pool reserve.
        reserves.insert(pair.clone(), reserve);
    }

    // Deduct the protocol fee from the output amount.
    let protocol_fee = output.amount.checked_mul_dec_ceil(protocol_fee_rate)?;
    output.amount.checked_sub_assign(protocol_fee)?;

    Ok((reserves, output, protocol_fee))
}

pub fn swap_exact_amount_out(
    storage: &dyn Storage,
    oracle_querier: &mut OracleQuerier<'_>,
    protocol_fee_rate: Udec128,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin, Uint128)> {
    let mut reserves = HashMap::new();
    let mut input = output.into_inner();

    for pair in route.into_iter().rev() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pair's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Perform the swap.
        (reserve, input) = params.swap_exact_amount_out(
            oracle_querier,
            &pair.base_denom,
            &pair.quote_denom,
            reserve,
            input,
        )?;

        // Save the updated reserves.
        reserves.insert(pair.clone(), reserve);
    }

    // Apply the protocol fee to the input amount.
    let protocol_fee = input.amount.checked_mul_dec_ceil(protocol_fee_rate)?;
    input.amount.checked_add_assign(protocol_fee)?;

    Ok((reserves, input, protocol_fee))
}
