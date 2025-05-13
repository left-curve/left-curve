use {
    crate::{DEBTS, MARKETS, core},
    dango_types::lending::Market,
    grug::{Addr, Coin, Coins, Denom, Number, QuerierWrapper, Storage, Timestamp, Udec256},
    std::collections::BTreeMap,
};

/// ## Returns
///
/// - Updated user scaled debts.
/// - Updated markets.
/// - Excess funds to be returned to the user.
pub fn repay(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Udec256>, Vec<(Denom, Market)>, Coins)> {
    let mut scaled_debts = DEBTS.may_load(storage, sender)?.unwrap_or_default();
    let mut markets = Vec::with_capacity(coins.len());
    let mut refunds = Coins::new();

    for coin in coins {
        // Update the market indices
        let market = MARKETS.load(storage, coin.denom)?;
        let market = core::update_indices(market, querier, current_time)?;

        // Calculated the users real debt
        let scaled_debt = scaled_debts.get(coin.denom).cloned().unwrap_or_default();
        let debt = core::into_underlying_debt(scaled_debt, &market)?;

        // Calculate the repaid amount and refund the remainders to the sender,
        // if any.
        let repaid = if coin.amount > &debt {
            let refund_amount = coin.amount.checked_sub(debt)?;
            refunds.insert(Coin::try_new(coin.denom.clone(), refund_amount)?)?;
            debt
        } else {
            *coin.amount
        };

        // If the repaid amount is equal to the debt, remove the debt from the
        // sender's debts. Otherwise, update the sender's liabilities.
        if repaid == debt {
            scaled_debts.remove(coin.denom);
        } else {
            // Update the sender's liabilities
            let repaid_debt_scaled = core::into_scaled_debt(repaid, &market)?;

            scaled_debts.insert(
                coin.denom.clone(),
                scaled_debt.saturating_sub(repaid_debt_scaled),
            );
        }

        // Deduct the repaid scaled debt and save the updated market state
        let debt_after = debt.checked_sub(repaid)?;
        let debt_after_scaled = core::into_scaled_debt(debt_after, &market)?;
        let scaled_debt_diff = scaled_debt.checked_sub(debt_after_scaled)?;

        // Update the market's borrowed amount.
        let market = market.deduct_borrowed(scaled_debt_diff)?;

        // Save the updated market state
        markets.push((coin.denom.clone(), market));
    }

    Ok((scaled_debts, markets, refunds))
}
