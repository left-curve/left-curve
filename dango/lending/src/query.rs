use {
    crate::{DEBTS, MARKETS},
    anyhow::bail,
    dango_types::lending::{Market, QueryMsg, NAMESPACE, SUBNAMESPACE},
    grug::{
        Addr, Bound, Coin, Coins, Denom, ImmutableCtx, Inner, Json, JsonSerExt, Number, Order,
        StdResult, Storage, Timestamp, Udec128,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Market { denom } => {
            let res = query_market(ctx.storage, denom)?;
            res.to_json_value()
        },
        QueryMsg::Markets { start_after, limit } => {
            let res = query_markets(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Debt { account } => {
            let res = query_debt(ctx.storage, ctx.block.timestamp, account)?;
            res.to_json_value()
        },
        QueryMsg::Debts { start_after, limit } => {
            let res = query_debts(ctx.storage, ctx.block.timestamp, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::PreviewDeposit { underlying } => {
            let (coins, ..) = query_preview_deposit(ctx.storage, ctx.block.timestamp, underlying)?;
            coins.to_json_value()
        },
        QueryMsg::PreviewWithdraw { lp_tokens } => {
            let (coins, ..) = query_preview_withdraw(ctx.storage, ctx.block.timestamp, lp_tokens)?;
            coins.to_json_value()
        },
    }
    .map_err(|err| anyhow::anyhow!(err))
}

fn query_market(storage: &dyn Storage, denom: Denom) -> StdResult<Market> {
    MARKETS.load(storage, &denom)
}

fn query_markets(
    storage: &dyn Storage,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Market>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    MARKETS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_debt(storage: &dyn Storage, timestamp: Timestamp, account: Addr) -> anyhow::Result<Coins> {
    println!("query_debt timestamp: {:?}", timestamp);
    let scaled_debts = DEBTS.load(storage, account)?;
    let mut debts = Coins::new();
    for (denom, scaled_debt) in scaled_debts {
        let market = MARKETS.load(storage, &denom)?.update_indices(timestamp)?;
        let debt = market.calculate_debt(scaled_debt)?;
        debts.insert(Coin::new(denom, debt)?)?;
    }
    Ok(debts)
}

fn query_debts(
    storage: &dyn Storage,
    timestamp: Timestamp,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> anyhow::Result<BTreeMap<Addr, Coins>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    DEBTS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (account, scaled_debts) = res?;
            let debts = scaled_debts
                .iter()
                .map(|(denom, scaled_debt)| {
                    let market = MARKETS.load(storage, denom)?.update_indices(timestamp)?;
                    let debt = market.calculate_debt(*scaled_debt)?;
                    Ok(Coin::new(denom.clone(), debt)?)
                })
                .collect::<Result<Vec<Coin>, anyhow::Error>>()?;
            Ok((account, Coins::try_from(debts)?))
        })
        .collect()
}

pub fn query_preview_deposit(
    storage: &dyn Storage,
    timestamp: Timestamp,
    underlying: Coins,
) -> anyhow::Result<(Coins, BTreeMap<Denom, Market>)> {
    let mut lp_tokens = Coins::new();
    let mut markets = BTreeMap::new();

    for coin in underlying {
        // Get market and update the market indices
        let market = MARKETS
            .load(storage, &coin.denom)?
            .update_indices(timestamp)?;

        // Compute the amount of LP tokens to mint
        let supply_index = market.supply_index;
        let amount_scaled = Udec128::new(coin.amount.into_inner())
            .checked_div(supply_index)?
            .into_int();

        let market = market.add_supplied(amount_scaled)?;
        lp_tokens.insert(Coin::new(market.supply_lp_denom.clone(), amount_scaled)?)?;
        markets.insert(coin.denom, market);
    }

    Ok((lp_tokens, markets))
}

pub fn query_preview_withdraw(
    storage: &dyn Storage,
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
            .update_indices(timestamp)?;

        let market = market.deduct_supplied(coin.amount)?;

        // Compute the amount of underlying coins to withdraw
        let supply_index = market.supply_index;
        let underlying_amount = Udec128::new(coin.amount.into_inner())
            .checked_mul(supply_index)?
            .into_int();

        withdrawn.insert(Coin::new(underlying_denom.clone(), underlying_amount)?)?;
        markets.insert(underlying_denom, market);
    }

    Ok((withdrawn, markets))
}
