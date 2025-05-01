use {
    crate::{ASSETS, MARKETS, core},
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Number, Storage, Timestamp, Uint128},
    std::collections::BTreeMap,
};

pub fn deposit(
    storage: &dyn Storage,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Uint128>, Vec<(Denom, Market)>)> {
    let mut markets = Vec::with_capacity(coins.len());
    let mut scaled_assets = ASSETS.may_load(storage, sender)?.unwrap_or_default();

    for coin in coins {
        // Get market and update the market indices
        let market = MARKETS.load(storage, coin.denom)?;
        let mut market = core::update_indices(market, current_time)?;

        // Update the user's asset.
        let diff = {
            // Find the user's current scaled asset.
            let scaled_before = scaled_assets.get(coin.denom).copied().unwrap_or_default();

            // Convert the user's current scaled asset from scaled to underlying.
            let underlying_before = core::scaled_asset_to_underlying(scaled_before, &market)?;

            // Increase the user's underlying asset amount.
            let underlying_after = underlying_before.checked_add(*coin.amount)?;

            // Convert the user's updated asset from underlying to scaled.
            let scaled_after = core::underlying_asset_to_scaled(underlying_after, &market)?;

            scaled_assets.insert(coin.denom.clone(), scaled_after);

            scaled_after - scaled_before
        };

        // Update the market's total asset.
        {
            market.total_supplied_scaled.checked_add_assign(diff)?;

            markets.push((coin.denom.clone(), market));
        }
    }

    Ok((scaled_assets, markets))
}
