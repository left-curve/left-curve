use {
    crate::{DEBTS, MARKETS, core},
    dango_types::lending::Market,
    grug::{Addr, Coin, Coins, Denom, Number, Storage, Timestamp, Uint128},
    std::{cmp::Ordering, collections::BTreeMap},
};

/// ## Returns
///
/// - Updated user scaled debts.
/// - Updated markets.
/// - Excess funds to be returned to the user.
pub fn repay(
    storage: &dyn Storage,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Uint128>, Vec<(Denom, Market)>, Coins)> {
    let mut markets = Vec::with_capacity(coins.len());
    let mut scaled_debts = DEBTS.may_load(storage, sender)?.unwrap_or_default();
    let mut refunds = Coins::new();

    for coin in coins {
        // Load and update the market state.
        let market = MARKETS.load(storage, coin.denom)?;
        let mut market = core::update_indices(market, current_time)?;

        // Update the user's debt; refund if necessary.
        let diff = {
            // Find the user's current scaled debt.
            let scaled_before = scaled_debts.get(coin.denom).copied().unwrap_or_default();

            // Convert the user's current scaled debt from scaled to underlying.
            let underlying_before = core::scaled_debt_to_underlying(scaled_before, &market)?;

            // Decrease the user's underlying debt amount.
            match underlying_before.cmp(coin.amount) {
                // User pays exactly the full amount.
                Ordering::Equal => {
                    // Clear the debt.
                    scaled_debts.remove(coin.denom);

                    scaled_before
                },
                // User pays more than the full amount.
                Ordering::Less => {
                    // Clear the debt.
                    scaled_debts.remove(coin.denom);

                    // Refund the excess.
                    let excess = *coin.amount - underlying_before;
                    refunds.insert(Coin::new(coin.denom.clone(), excess)?)?;

                    scaled_before
                },
                // User pays less than the full amount.
                Ordering::Greater => {
                    // Decrease the user's underlying debt amount.
                    let underlying_after = underlying_before.checked_sub(*coin.amount)?;

                    // Convert the user's updated debt from underlying to scaled.
                    let scaled_after = core::underlying_debt_to_scaled(underlying_after, &market)?;

                    // Update the user's scaled debt.
                    scaled_debts.insert(coin.denom.clone(), scaled_after);

                    scaled_before - scaled_after
                },
            }
        };

        // Update the market's total debt.
        {
            market.total_borrowed_scaled.checked_sub_assign(diff)?;

            markets.push((coin.denom.clone(), market));
        }
    }

    Ok((scaled_debts, markets, refunds))
}
