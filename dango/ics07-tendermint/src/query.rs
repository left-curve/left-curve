//! Defines the query interface for the ics07-tendermint contract.

use grug::{ImmutableCtx, Json, StdResult};

/// The query messages for the contract.
#[grug::derive(Serde)]
#[allow(clippy::module_name_repetitions)]
pub enum QueryMsg {
    // TODO: Replace this with the actual query messages.
}

/// The query entrypoint for the contract.
/// # Errors
/// Returns an error if the underlying query handler encounters an error.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn query(_ctx: ImmutableCtx, _msg: QueryMsg) -> StdResult<Json> {
    todo!()
}
