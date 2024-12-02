use {
    anyhow::Result,
    dango_types::ibc_client::{ExecuteMsg, InstantiateMsg},
    grug::{MutableCtx, Response},
};

use crate::ctx::TendermintContext;

/// The instantiate entrypoint for the contract.
/// # Errors
/// Returns an error if the contract encounters an error during instantiation.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> Result<Response> {
    TendermintContext::new_mut(ctx)?.instantiate(msg)?;

    Ok(Response::default())
}

/// Execute function is called when a contract is invoked with a message.
/// # Errors
/// Returns an error if the underlying message handler encounters an error.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    TendermintContext::new_mut(ctx)?.execute(msg)?;

    Ok(Response::default())
}
