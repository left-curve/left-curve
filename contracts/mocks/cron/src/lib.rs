#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use cw_std::{AfterBlockCtx, BeforeBlockCtx, Empty, InstantiateCtx, Response, StdResult};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(_ctx: InstantiateCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn before_block(_ctx: BeforeBlockCtx) -> StdResult<Response> {
    // nothing to do
    Ok(Response::new().add_attribute("method", "before_block"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn after_block(_ctx: AfterBlockCtx) -> StdResult<Response> {
    // nothing to do
    Ok(Response::new().add_attribute("method", "after_block"))
}
