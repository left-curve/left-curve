use {
    dango_auth::NEXT_NONCE,
    dango_types::account::spot::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Nonce {} => query_nonce(ctx.storage)?.to_json_value(),
    }
}

fn query_nonce(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_NONCE.current(storage)
}
