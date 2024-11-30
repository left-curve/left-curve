use dango_types::ibc_client::{ExecuteMsg, InstantiateMsg};
use grug::{MutableCtx, Response, StdResult};

/// The instantiate entrypoint for the contract.
/// # Errors
/// Returns an error if the contract encounters an error during instantiation.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    todo!()
}

/// Execute function is called when a contract is invoked with a message.
/// # Errors
/// Returns an error if the underlying message handler encounters an error.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn execute(_ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateClient(_) => todo!(),
        ExecuteMsg::Misbehaviour(_) => todo!(),
        ExecuteMsg::UpgradeClient(_) => todo!(),
    }
}
