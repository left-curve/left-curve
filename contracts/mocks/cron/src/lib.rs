#[cfg(not(feature = "library"))]
use cw_std::entry_point;
use cw_std::{Empty, MutableCtx, Response, StdResult, SudoCtx};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn before_block(_ctx: SudoCtx) -> StdResult<Response> {
    // nothing to do
    Ok(Response::new().add_attribute("method", "before_block"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn after_block(_ctx: SudoCtx) -> StdResult<Response> {
    // nothing to do
    Ok(Response::new().add_attribute("method", "after_block"))
}
