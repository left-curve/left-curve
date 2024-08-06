use {
    crate::{
        force_write_on_query, infinite_loop, query_force_write, query_loop, ExecuteMsg,
        InstantiateMsg, QueryMsg,
    },
    grug::{to_json_value, ImmutableCtx, Json, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::InfiniteLoop {} => infinite_loop(),
        ExecuteMsg::ForceWriteOnQuery { key, value } => force_write_on_query(ctx, key, value),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(_ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Loop { iterations } => to_json_value(&query_loop(iterations)?),
        QueryMsg::ForceWrite { key, value } => to_json_value(&query_force_write(&key, &value)),
    }
}
