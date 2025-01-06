use {
    crate::MarginQuerier,
    dango_auth::query_nonce,
    dango_types::account::margin::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt},
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
