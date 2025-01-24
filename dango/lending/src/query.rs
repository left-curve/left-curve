use {
    crate::{DEBTS, MARKETS},
    dango_types::lending::{Market, QueryMsg},
    grug::{
        Addr, Bound, Coin, Coins, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage,
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
            let res = query_debt(ctx.storage, account)?;
            res.to_json_value()
        },
        QueryMsg::Debts { start_after, limit } => {
            let res = query_debts(ctx.storage, start_after, limit)?;
            res.to_json_value()
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

fn query_debt(storage: &dyn Storage, account: Addr) -> anyhow::Result<Coins> {
    let scaled_debts = DEBTS.load(storage, account)?;
    let mut debts = Coins::new();
    for (denom, scaled_debt) in scaled_debts {
        let market = MARKETS.load(storage, &denom)?;
        let debt = market.calculate_debt(scaled_debt)?;
        debts.insert(Coin::new(denom, debt)?)?;
    }
    Ok(debts)
}

fn query_debts(
    storage: &dyn Storage,
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
                    let market = MARKETS.load(storage, denom)?;
                    let debt = market.calculate_debt(*scaled_debt)?;
                    Ok(Coin::new(denom.clone(), debt)?)
                })
                .collect::<Result<Vec<Coin>, anyhow::Error>>()?;
            Ok((account, Coins::try_from(debts)?))
        })
        .collect()
}
