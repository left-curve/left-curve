use {
    crate::{OracleQuerier, GUARDIAN_SETS, PRICE_SOURCES},
    dango_types::oracle::{GuardianSet, GuardianSetIndex, PrecisionedPrice, PriceSource, QueryMsg},
    grug::{Addressable, Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Price { denom } => {
            let res = query_price(ctx, denom)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::Prices { start_after, limit } => {
            let res = query_prices(ctx, start_after, limit)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::PriceSource { denom } => {
            let res = query_price_source(ctx, denom)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::PriceSources { start_after, limit } => {
            let res = query_price_sources(ctx, start_after, limit)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::GuardianSet { index } => {
            let res = query_guardian_set(ctx, index)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::GuardianSets { start_after, limit } => {
            let res = query_guardian_sets(ctx, start_after, limit)?;
            Ok(res.to_json_value()?)
        },
    }
}

fn query_price(ctx: ImmutableCtx, denom: Denom) -> anyhow::Result<PrecisionedPrice> {
    ctx.querier
        .query_price(ctx.contract.address(), &denom, None)
}

fn query_prices(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> anyhow::Result<BTreeMap<Denom, PrecisionedPrice>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PRICE_SOURCES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            let (denom, price_source) = res?;
            let price =
                ctx.querier
                    .query_price(ctx.contract.address(), &denom, Some(price_source))?;
            Ok((denom, price))
        })
        .collect()
}

fn query_price_source(ctx: ImmutableCtx, denom: Denom) -> StdResult<PriceSource> {
    PRICE_SOURCES.load(ctx.storage, &denom)
}

fn query_price_sources(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, PriceSource>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PRICE_SOURCES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_guardian_set(ctx: ImmutableCtx, index: u32) -> StdResult<GuardianSet> {
    GUARDIAN_SETS.load(ctx.storage, index)
}

fn query_guardian_sets(
    ctx: ImmutableCtx,
    start_after: Option<GuardianSetIndex>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<GuardianSetIndex, GuardianSet>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    GUARDIAN_SETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}
