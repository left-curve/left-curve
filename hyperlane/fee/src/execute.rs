use {
    crate::MAILBOX,
    anyhow::ensure,
    grug::{HexBinary, MutableCtx, Response, StdResult},
    hyperlane_types::fee::{ExecuteMsg, InstantiateMsg},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::PostDispatch { raw_message, .. } => post_dispatch(ctx, raw_message),
    }
}

#[inline]
fn post_dispatch(ctx: MutableCtx, _raw_message: HexBinary) -> anyhow::Result<Response> {
    // In the reference implementation, we should check here that the message ID
    // matches the mailbox's last dispatched ID.
    // Here instead, we just ensure the sender is the mailbox, and trust the
    // mailbox is properly implemented, i.e. it only calls this right after
    // dispatching a message.
    ensure!(
        ctx.sender == MAILBOX.load(ctx.storage)?,
        "sender is not mailbox"
    );

    // TODO

    Ok(Response::new())
}
