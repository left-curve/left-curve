use {
    crate::{perform_swap, CONFIG, POOLS},
    dango_types::amm::{Config, Pool, PoolId, QueryMsg, SwapOutcome},
    grug::{Bound, Coin, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage, UniqueVec},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Pool { pool_id } => {
            let res = query_pool(ctx.storage, pool_id)?;
            res.to_json_value()
        },
        QueryMsg::Pools { start_after, limit } => {
            let res = query_pools(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Simulate { input, route } => {
            let res = query_simulte(ctx.storage, input, route)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

fn query_pool(storage: &dyn Storage, pool_id: PoolId) -> StdResult<Pool> {
    POOLS.load(storage, pool_id)
}

fn query_pools(
    storage: &dyn Storage,
    start_after: Option<PoolId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PoolId, Pool>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    POOLS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_simulte(
    storage: &dyn Storage,
    input: Coin,
    route: UniqueVec<PoolId>,
) -> anyhow::Result<SwapOutcome> {
    let cfg = CONFIG.load(storage)?;
    let mut pools = route
        .into_iter()
        .map(|pool_id| POOLS.load(storage, pool_id))
        .collect::<StdResult<Vec<_>>>()?;

    perform_swap(&cfg, input, pools.iter_mut())
}
