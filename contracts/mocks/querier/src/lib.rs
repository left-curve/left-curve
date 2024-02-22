#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use cw_std::{
    cw_derive, to_json, Binary, Empty, ExecuteCtx, InstantiateCtx, QueryCtx, QueryRequest,
    ReceiveCtx, Response, StdResult,
};

#[cw_derive(serde)]
pub enum QueryMsg {
    QueryChain {
        request: QueryRequest,
    },
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(_ctx: InstantiateCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn receive(_ctx: ReceiveCtx) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_ctx: ExecuteCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryChain {
            request,
        } => to_json(&ctx.query(&request)?),
    }
}
