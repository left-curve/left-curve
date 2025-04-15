use {
    crate::{DEBTS, MARKETS},
    dango_types::lending::Market,
    grug::{Addr, Coin, Coins, Denom, NextNumber, Number, Storage, Timestamp, Udec256},
    std::collections::BTreeMap,
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
    funds: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Udec256>, Vec<(Denom, Market)>, Coins)> {
    let mut scaled_debts = DEBTS.may_load(storage, sender)?.unwrap_or_default();
    let mut markets = Vec::with_capacity(funds.len());
    let mut refunds = Coins::new();

    for coin in funds {
        // Update the market indices
        let market = MARKETS
            .load(storage, coin.denom)?
            .update_indices(current_time)?;

        // Calculated the users real debt
        let scaled_debt = scaled_debts.get(coin.denom).cloned().unwrap_or_default();
        let debt = market.calculate_debt(scaled_debt)?;

        // Calculate the repaid amount and refund the remainders to the sender,
        // if any.
        let repaid = if coin.amount > &debt {
            let refund_amount = coin.amount.checked_sub(debt)?;
            refunds.insert(Coin::new(coin.denom.clone(), refund_amount)?)?;
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
            let repaid_debt_scaled = repaid
                .into_next()
                .checked_into_dec()?
                .checked_div(market.borrow_index.into_next())?;

            scaled_debts.insert(
                coin.denom.clone(),
                scaled_debt.saturating_sub(repaid_debt_scaled),
            );
        }

        // Deduct the repaid scaled debt and save the updated market state
        let debt_after = debt.checked_sub(repaid)?;
        let debt_after_scaled = debt_after
            .into_next()
            .checked_into_dec()?
            .checked_div(market.borrow_index.into_next())?;
        let scaled_debt_diff = scaled_debt.checked_sub(debt_after_scaled)?;

        // Update the market's interest rates.
        let market = market
            .deduct_borrowed(scaled_debt_diff)?
            .update_interest_rates()?;

        // Save the updated market state
        markets.push((coin.denom.clone(), market));
    }

    Ok((scaled_debts, markets, refunds))
}
