use {
    crate::{ASSETS, MARKETS, core},
    anyhow::bail,
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Number, Storage, Timestamp, Uint128},
    std::{cmp::Ordering, collections::BTreeMap},
};

pub fn withdraw(
    storage: &dyn Storage,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Uint128>, Vec<(Denom, Market)>)> {
    let mut markets = Vec::with_capacity(coins.len());
    let mut scaled_assets = ASSETS.may_load(storage, sender)?.unwrap_or_default();

    for coin in coins {
        // Load and update the market state.
        let market = MARKETS.load(storage, coin.denom)?;
        let mut market = core::update_indices(market, current_time)?;

        // Update the user's asset.
        let diff = {
            // Find the user's current scaled asset.
            let scaled_before = scaled_assets.get(coin.denom).copied().unwrap_or_default();

            // Convert the user's current scaled asset from scaled to underlying.
            let underlying_before = core::scaled_asset_to_underlying(scaled_before, &market)?;

            // Decrease the user's underlying asset amount.
            match underlying_before.cmp(coin.amount) {
                // User withdraws exactly the full amount.
                Ordering::Equal => {
                    // Clear the asset.
                    scaled_assets.remove(coin.denom);

                    scaled_before
                },
                // User attempts to withdraw more than the full amount. Reject.
                Ordering::Less => {
                    bail!(
                        "can't withdraw more than the available deposited balance! available: {}, requested: {}",
                        underlying_before,
                        coin.amount
                    );
                },
                // User withdraws less than the full amount.
                Ordering::Greater => {
                    // Decrease the user's underlying asset amount.
                    let underlying_after = underlying_before.checked_sub(*coin.amount)?;

                    // Convert the user's updated asset from underlying to scaled.
                    let scaled_after = core::underlying_asset_to_scaled(underlying_after, &market)?;

                    scaled_assets.insert(coin.denom.clone(), scaled_after);

                    scaled_before - scaled_after
                },
            }
        };

        // Update the market's total asset.
        {
            market.total_supplied_scaled.checked_sub_assign(diff)?;

            markets.push((coin.denom.clone(), market));
        }
    }

    Ok((scaled_assets, markets))
}
