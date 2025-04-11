use {
    crate::MARKETS,
    anyhow::bail,
    dango_types::lending::{Market, NAMESPACE, SUBNAMESPACE},
    grug::{Coin, Coins, Denom, MultiplyFraction, QuerierWrapper, Storage, Timestamp},
    std::collections::BTreeMap,
};

/// Calculates the amount of LP tokens to mint for a deposit.
/// Returns the amount of LP tokens and the updated markets.
pub fn deposit(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    timestamp: Timestamp,
    underlying: Coins,
) -> anyhow::Result<(Coins, BTreeMap<Denom, Market>)> {
    let mut lp_tokens = Coins::new();
    let mut markets = BTreeMap::new();

    for coin in underlying {
        // Get market and update the market indices
        let market = MARKETS
            .load(storage, &coin.denom)?
            .update_indices(querier, timestamp)?;

        // Compute the amount of LP tokens to mint
        let supply_index = market.supply_index;
        let amount_scaled = coin.amount.checked_div_dec_floor(supply_index)?;
        lp_tokens.insert(Coin::new(market.supply_lp_denom.clone(), amount_scaled)?)?;
        markets.insert(coin.denom, market);
    }

    Ok((lp_tokens, markets))
}

/// Calculates the amount of underlying coins to withdraw for a given amount of LP tokens.
/// Returns the amount of underlying coins and the updated markets.
pub fn withdraw(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    timestamp: Timestamp,
    lp_tokens: Coins,
) -> anyhow::Result<(Coins, BTreeMap<Denom, Market>)> {
    let mut withdrawn = Coins::new();
    let mut markets = BTreeMap::new();

    for coin in lp_tokens {
        let Some(underlying_denom) = coin.denom.strip(&[&NAMESPACE, &SUBNAMESPACE]) else {
            bail!("not a lending pool token: {}", coin.denom)
        };

        // Update the market indices
        let market = MARKETS
            .load(storage, &underlying_denom)?
            .update_indices(querier, timestamp)?;

        // Compute the amount of underlying coins to withdraw
        let underlying_amount = coin.amount.checked_mul_dec_floor(market.supply_index)?;
        withdrawn.insert(Coin::new(underlying_denom.clone(), underlying_amount)?)?;
        markets.insert(underlying_denom, market);
    }

    Ok((withdrawn, markets))
}
