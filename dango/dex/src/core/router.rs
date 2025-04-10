use {
    crate::{PAIRS, PassiveLiquidityPool, RESERVES},
    dango_types::dex::{PairId, SwapExactAmountIn, SwapExactAmountOut},
    grug::{Coin, CoinPair, EventBuilder, Inner, NonZero, Storage, UniqueVec},
    std::collections::HashMap,
};

pub fn swap_exact_amount_in(
    storage: &dyn Storage,
    route: UniqueVec<PairId>,
    input: Coin,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin, EventBuilder)> {
    let mut reserves = HashMap::new();
    let mut output = input;

    let mut events = EventBuilder::new();

    for pair in route.into_iter() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pool's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Clone the input for the event.
        let input = output.clone();

        // Perform the swap.
        // The output of the previous step is the input of this step.
        (reserve, output) = params.swap_exact_amount_in(reserve, output)?;

        // Save the updated pool reserve.
        reserves.insert(pair.clone(), reserve);

        // Add events
        events.push(SwapExactAmountIn {
            pair,
            input,
            output: output.clone(),
        })?;
    }

    Ok((reserves, output, events))
}

pub fn swap_exact_amount_out(
    storage: &dyn Storage,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<(HashMap<PairId, CoinPair>, Coin, EventBuilder)> {
    let mut reserves = HashMap::new();
    let mut input = output.into_inner();
    let mut events = EventBuilder::new();

    for pair in route.into_iter().rev() {
        // Load the pair's parameters.
        let params = PAIRS.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Load the pair's reserves.
        let mut reserve = RESERVES.load(storage, (&pair.base_denom, &pair.quote_denom))?;

        // Clone the output for the event.
        let output = input.clone();

        // Perform the swap.
        (reserve, input) = params.swap_exact_amount_out(reserve, input)?;

        // Save the updated reserves.
        reserves.insert(pair.clone(), reserve);

        // Add event
        events.push(SwapExactAmountOut {
            pair,
            input: input.clone(),
            output,
        })?;
    }

    Ok((reserves, input, events))
}
