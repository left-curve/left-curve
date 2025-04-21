use {
    crate::{MARKETS, core},
    dango_types::lending::Market,
    grug::{Coin, Coins, Denom, MultiplyFraction, QuerierWrapper, Storage, Timestamp},
    std::collections::BTreeMap,
};

/// Calculates the amount of LP tokens to mint for a deposit.
/// Returns the amount of LP tokens and the updated markets.
pub fn deposit(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    current_time: Timestamp,
    coins: Coins,
) -> anyhow::Result<(Coins, BTreeMap<Denom, Market>)> {
    let mut lp_tokens = Coins::new();
    let mut markets = BTreeMap::new();

    for coin in coins {
        // Get market and update the market indices
        let market = MARKETS.load(storage, &coin.denom)?;
        let market = core::update_indices(market, querier, current_time)?;

        // Compute the amount of LP tokens to mint
        let supply_index = market.supply_index;
        let amount_scaled = coin.amount.checked_div_dec_floor(supply_index)?;
        lp_tokens.insert(Coin::new(market.supply_lp_denom.clone(), amount_scaled)?)?;

        // Save the updated market state.
        markets.insert(coin.denom, market);
    }

    Ok((lp_tokens, markets))
}
