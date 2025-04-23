use {
    crate::{DEBTS, MARKETS, core},
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Number, QuerierWrapper, Storage, Timestamp, Udec256},
    std::collections::BTreeMap,
};

pub fn borrow(
    storage: &dyn Storage,
    querier: QuerierWrapper,
    current_time: Timestamp,
    sender: Addr,
    coins: &Coins,
) -> anyhow::Result<(BTreeMap<Denom, Udec256>, Vec<(Denom, Market)>)> {
    let mut markets = Vec::with_capacity(coins.len());
    let mut scaled_debts = DEBTS.may_load(storage, sender)?.unwrap_or_default();

    for coin in coins {
        // Update the market state
        let market = MARKETS.load(storage, coin.denom)?;
        let market = core::update_indices(market, querier, current_time)?;

        // Update the sender's liabilities
        let prev_scaled_debt = scaled_debts.get(coin.denom).cloned().unwrap_or_default();
        let new_scaled_debt = core::into_scaled_debt(*coin.amount, &market)?;
        let added_scaled_debt = prev_scaled_debt.checked_add(new_scaled_debt)?;
        scaled_debts.insert(coin.denom.clone(), added_scaled_debt);

        // Update the market's borrowed amount.
        let market = market.add_borrowed(added_scaled_debt)?;

        // Save the updated market state.
        markets.push((coin.denom.clone(), market));
    }

    Ok((scaled_debts, markets))
}
