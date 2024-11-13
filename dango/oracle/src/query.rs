use {
    crate::{GUARDIAN_SETS, PRICE_SOURCES},
    dango_types::{
        config::AppConfig,
        oracle::{GuardianSet, PrecisionedPrice, PriceSource, QueryMsg},
    },
    grug::{
        BorshDeExt, Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, QuerierWrapper, StdResult,
    },
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
    PRICE_SOURCES
        .load(ctx.storage, &denom)?
        .get_price(ctx.storage)
}

/// Does a raw query to the oracle contract to get the latest price for the given denom.
pub fn raw_query_price(
    querier: &QuerierWrapper,
    denom: &Denom,
) -> anyhow::Result<PrecisionedPrice> {
    let app_cfg: AppConfig = querier.query_app_config()?;
    let oracle = app_cfg.addresses.oracle;

    let price = querier
        .query_wasm_raw(oracle, PRICE_SOURCES.path(denom))?
        .ok_or(anyhow::anyhow!(
            "Price source not found for denom: {}",
            denom
        ))?
        .deserialize_borsh::<PriceSource>()?
        .raw_query_price(querier, oracle)?;

    Ok(price)
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
            let price = price_source.get_price(ctx.storage)?;
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
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<u32, GuardianSet>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    GUARDIAN_SETS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}
