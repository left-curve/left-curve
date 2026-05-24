use {
    crate::{ExecuteMsg, QueryMsg, query_state, update_key},
    grug_types::{ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateKey { new_public_key } => update_key(ctx, &new_public_key),
    }
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::State {} => query_state(ctx.storage)?.to_json_value(),
    }
}
