use grug::{MutableCtx, Response, StdResult};

/// The execute entrypoint for the contract.
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    // TODO: remove this example and add your own custom msg types
}

/// The execute entrypoint for the contract.
#[grug::derive(Serde)]
#[allow(clippy::module_name_repetitions)]
pub enum ExecuteMsg {
    // TODO: remove this example and add your own custom msg types
}

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
pub fn execute(_ctx: MutableCtx, _msg: ExecuteMsg) -> anyhow::Result<Response> {
    todo!()
}
