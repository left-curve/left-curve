use {
    crate::MARKETS,
    anyhow::bail,
    dango_types::lending::{Market, NAMESPACE, SUBNAMESPACE},
    grug::{Coin, Coins, Denom, MultiplyFraction, Storage, Timestamp},
    std::collections::BTreeMap,
};

/// Calculates the amount of underlying coins to withdraw for a given amount of LP tokens.
/// Returns the amount of underlying coins and the updated markets.
pub fn withdraw(
    storage: &dyn Storage,
    current_time: Timestamp,
    coins: Coins,
) -> anyhow::Result<(Coins, BTreeMap<Denom, Market>)> {
    let mut withdrawn = Coins::new();
    let mut markets = BTreeMap::new();

    for coin in coins {
        let Some(underlying_denom) = coin.denom.strip(&[&NAMESPACE, &SUBNAMESPACE]) else {
            bail!("not a lending pool token: {}", coin.denom)
        };

        // Update the market indices
        let market = MARKETS
            .load(storage, &underlying_denom)?
            .update_indices(current_time)?;

        // Compute the amount of underlying coins to withdraw
        let underlying_amount = coin.amount.checked_mul_dec_floor(market.supply_index)?;
        withdrawn.insert(Coin::new(underlying_denom.clone(), underlying_amount)?)?;

        // Update the market's interest rates.
        let market = market
            .deduct_supplied(coin.amount)?
            .update_interest_rates()?;

        // Save the updated market state
        markets.insert(underlying_denom, market);
    }

    Ok((withdrawn, markets))
}
