use {
    crate::{PAIRS, PassiveLiquidityPool, RESERVES},
    dango_types::dex::PairId,
    grug::{Coin, CoinPair, Inner, NonZero, Storage, UniqueVec},
    std::collections::HashMap,
};

pub fn swap_exact_amount_in(
    storage: &dyn Storage,
    route: UniqueVec<PairId>,
    input: Coin,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin)> {
    let mut reserves = HashMap::new();
    let mut output = input;

    for pair in route.into_iter() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pool's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Perform the swap.
        // The output of the previous step is the input of this step.
        (reserve, output) = params.swap_exact_amount_in(reserve, output)?;

        // Save the updated pool reserve.
        reserves.insert(pair.clone(), reserve);
    }

    Ok((reserves, output))
}

pub fn swap_exact_amount_out(
    storage: &dyn Storage,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin)> {
    let mut reserves = HashMap::new();
    let mut input = output.into_inner();

    for pair in route.into_iter().rev() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pair's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Perform the swap.
        (reserve, input) = params.swap_exact_amount_out(reserve, input)?;

        // Save the updated reserves.
        reserves.insert(pair.clone(), reserve);
    }

    Ok((reserves, input))
}
