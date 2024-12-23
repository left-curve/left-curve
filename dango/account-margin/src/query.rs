use {
    crate::MarginQuerier,
    dango_auth::NEXT_NONCE,
    dango_types::account::margin::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Nonce {} => {
            let res = query_nonce(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Health {} => {
            let res = ctx.querier.query_health(ctx.contract)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_nonce(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_NONCE.current(storage)
}
