use {
    dango_auth::query_seen_nonces,
    dango_types::account::spot::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
    }
}
