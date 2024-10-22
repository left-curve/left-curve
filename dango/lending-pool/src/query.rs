use {
    crate::WHITELISTED_DENOMS,
    dango_types::lending_pool::QueryMsg,
    grug::{Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::WhitelistedDenoms { limit, start_after } => {
            let res = query_whitelisted_denoms(ctx.storage, limit, start_after)?;
            res.to_json_value()
        },
    }
}

pub fn query_whitelisted_denoms(
    storage: &dyn Storage,
    limit: Option<u32>,
    start_after: Option<Denom>,
) -> StdResult<Vec<Denom>> {
    let iter = WHITELISTED_DENOMS
        .range(
            storage,
            start_after.map(Bound::Exclusive),
            None,
            Order::Descending,
        )
        .into_iter();

    let res = if let Some(limit) = limit {
        iter.take(limit as usize).collect::<StdResult<Vec<_>>>()?
    } else {
        iter.collect::<StdResult<Vec<_>>>()?
    };

    Ok(res)
}
