use {
    crate::{InstantiateMsg, QueryMsg, initialize_config, query_config},
    grug_types::{ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize_config(ctx.storage, &msg.config)
}

#[cfg_attr(not(feature = "library"), grug_ffi::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => query_config(ctx.storage)?.to_json_value(),
    }
}
