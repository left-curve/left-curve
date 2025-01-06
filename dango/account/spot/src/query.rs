use {
    dango_auth::SEEN_NONCES,
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
    let nonces = SEEN_NONCES.load(storage).unwrap_or_default();
    Ok(nonces.last().map(|&nonce| nonce + 1).unwrap_or(0))
}
