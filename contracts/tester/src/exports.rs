use {
    crate::{
        exeucte_stack_overflow, force_write_on_query, infinite_loop, query_force_write, query_loop,
        query_stack_overflow, ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    grug::{ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
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
        ExecuteMsg::StackOverflow {} => exeucte_stack_overflow(ctx),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Loop { iterations } => query_loop(iterations)?.to_json_value(),
        QueryMsg::ForceWrite { key, value } => query_force_write(&key, &value).to_json_value(),
        QueryMsg::StackOverflow {} => query_stack_overflow(ctx)?.to_json_value(),
    }
}
