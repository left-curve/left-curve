use cw_std::{
    cw_serde, entry_point, to_json, Binary, Empty, ExecuteCtx, InstantiateCtx, QueryCtx,
    QueryRequest, Response, StdResult,
};

#[cw_serde]
pub enum QueryMsg {
    QueryChain {
        request: QueryRequest,
    },
}

#[entry_point]
pub fn instantiate(_ctx: InstantiateCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(_ctx: ExecuteCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryChain {
            request,
        } => to_json(&ctx.query(&request)?),
    }
}
