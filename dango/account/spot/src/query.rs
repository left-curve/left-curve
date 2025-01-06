use {
    dango_auth::query_nonce,
    dango_types::account::spot::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Nonce {} => query_nonce(ctx.storage)?.to_json_value(),
    }
}
