use {
    crate::{DEBTS, MARKETS, core},
    dango_types::lending::{Market, QueryMsg},
    grug::{
        Addr, Bound, Coins, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order,
        StdResult,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Market { denom } => {
            let res = query_market(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::Markets { start_after, limit } => {
            let res = query_markets(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Debt { account } => {
            let res = query_debt(ctx, account)?;
            res.to_json_value()
        },
        QueryMsg::Debts { start_after, limit } => {
            let res = query_debts(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::SimulateDeposit { underlying } => {
            let (lp_tokens, _) =
                core::deposit(ctx.storage, &ctx.querier, ctx.block.timestamp, underlying)?;
            lp_tokens.to_json_value()
        },
        QueryMsg::SimulateWithdraw { lp_tokens } => {
            let (coins, _) =
                core::withdraw(ctx.storage, &ctx.querier, ctx.block.timestamp, lp_tokens)?;
            coins.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_market(ctx: ImmutableCtx, denom: Denom) -> StdResult<Market> {
    MARKETS.load(ctx.storage, &denom)
}

fn query_markets(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Market>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    MARKETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_debt(ctx: ImmutableCtx, account: Addr) -> anyhow::Result<Coins> {
    let coins = DEBTS
        .load(ctx.storage, account)?
        .into_iter()
        .map(|(denom, scaled_debt)| {
            let market = MARKETS
                .load(ctx.storage, &denom)?
                .update_indices(&ctx.querier, ctx.block.timestamp)?;
            let debt = market.calculate_debt(scaled_debt)?;

            Ok((denom, debt))
        })
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

    Ok(Coins::new_unchecked(coins))
}

fn query_debts(
    ctx: ImmutableCtx,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> anyhow::Result<BTreeMap<Addr, Coins>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    DEBTS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (account, scaled_debts) = res?;
            let debts = scaled_debts
                .into_iter()
                .map(|(denom, scaled_debt)| {
                    let market = MARKETS
                        .load(ctx.storage, &denom)?
                        .update_indices(&ctx.querier, ctx.block.timestamp)?;
                    let debt = market.calculate_debt(scaled_debt)?;

                    Ok((denom, debt))
                })
                .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

            Ok((account, Coins::new_unchecked(debts)))
        })
        .collect()
}
