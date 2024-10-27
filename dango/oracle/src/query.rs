use {
    crate::{CONFIG, GUARDIANS, PRICES, PRICE_FEEDS},
    dango_types::oracle::{Config, Price, QueryMsg},
    grug::{Addr, Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Udec128},
    std::collections::BTreeSet,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Guardians {} => {
            let res = query_guardians(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Price { denom } => {
            let res = query_price(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::Prices { start_after, limit } => {
            let res = query_prices(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::PriceFeed { denom, guardian } => {
            let res = query_price_feed(ctx, denom, guardian)?;
            res.to_json_value()
        },
        QueryMsg::PriceFeeds {
            denom,
            start_after,
            limit,
        } => {
            let res = query_price_feeds(ctx, denom, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}

fn query_guardians(ctx: ImmutableCtx) -> StdResult<BTreeSet<Addr>> {
    GUARDIANS
        .range(ctx.storage, None, None, Order::Ascending)
        .collect()
}

fn query_price(ctx: ImmutableCtx, denom: Denom) -> StdResult<Price> {
    PRICES.load(ctx.storage, &denom)
}

fn query_prices(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Vec<(Denom, Price)>> {
    let start_after = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PRICES
        .range(ctx.storage, start_after, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_price_feed(ctx: ImmutableCtx, denom: Denom, guardian: Addr) -> StdResult<Udec128> {
    PRICE_FEEDS.load(ctx.storage, (&denom, guardian))
}

fn query_price_feeds(
    ctx: ImmutableCtx,
    denom: Denom,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<Vec<(Addr, Udec128)>> {
    let start_after = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PRICE_FEEDS
        .prefix(&denom)
        .range(ctx.storage, start_after, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
