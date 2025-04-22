use {
    crate::{DEBTS, MARKETS, core},
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Number, Storage, Timestamp, Uint128},
    std::collections::BTreeMap,
};

pub fn borrow(
    storage: &dyn Storage,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Uint128>, Vec<(Denom, Market)>)> {
    let mut markets = Vec::with_capacity(coins.len());
    let mut scaled_debts = DEBTS.may_load(storage, sender)?.unwrap_or_default();

    for coin in coins {
        // Load and update the market state.
        let market = MARKETS.load(storage, coin.denom)?;
        let mut market = core::update_indices(market, current_time)?;

        // Update the user's debt.
        let diff = {
            // Find the user's current scaled debt.
            let scaled_before = scaled_debts.get(coin.denom).copied().unwrap_or_default();

            // Convert the user's current scaled debt from scaled to underlying.
            let underlying_before = core::scaled_debt_to_underlying(scaled_before, &market)?;

            // Increase the user's underlying debt amount.
            let underlying_after = underlying_before.checked_add(*coin.amount)?;

            // Convert the user's updated debt from underlying to scaled.
            let scaled_after = core::underlying_debt_to_scaled(underlying_after, &market)?;

            scaled_debts.insert(coin.denom.clone(), scaled_after);

            scaled_after - scaled_before
        };

        // Update the market's total debt.
        {
            market.total_borrowed_scaled.checked_add_assign(diff)?;

            markets.push((coin.denom.clone(), market));
        }
    }

    Ok((scaled_debts, markets))
}
