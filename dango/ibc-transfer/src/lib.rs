use {
    dango_types::ibc::transfer::{ExecuteMsg, InstantiateMsg},
    grug::{Addr, Message, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ReceiveTransfer { recipient } => receive_transfer(ctx, recipient),
    }
}

fn receive_transfer(ctx: MutableCtx, recipient: Addr) -> StdResult<Response> {
    Ok(Response::new().add_message(Message::transfer(recipient, ctx.funds)?))
}
