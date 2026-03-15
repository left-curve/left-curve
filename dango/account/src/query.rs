use {
    dango_auth::{query_seen_nonces, query_status},
    dango_types::account::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Status {} => {
            let res = query_status(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
    }
}
