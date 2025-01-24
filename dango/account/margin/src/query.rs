use {
    crate::MarginQuerier,
    dango_auth::query_seen_nonces,
    dango_types::account::margin::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Health {} => {
            let res = ctx.querier.query_health(ctx.contract, None)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}
