use {
    crate::PRICE_SOURCES,
    dango_types::oracle::{PrecisionedPrice, PriceSource, QueryMsg},
    grug::{Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::QueryPrice { denom } => {
            let res = query_price(ctx, denom)?;
            Ok(res.to_json_value()?)
        },
        QueryMsg::QueryPriceSources { start_after, limit } => {
            let res = query_price_sources(ctx, start_after, limit)?;
            Ok(res.to_json_value()?)
        },
    }
}

fn query_price(ctx: ImmutableCtx, denom: Denom) -> anyhow::Result<PrecisionedPrice> {
    PRICE_SOURCES
        .load(ctx.storage, &denom)?
        .get_price(ctx.storage)
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
