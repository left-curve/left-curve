//! Defines the query interface for the ics07-tendermint contract.

use dango_types::ibc_client::QueryMsg;
use grug::{ImmutableCtx, Json, StdResult};

/// The query entrypoint for the contract.
/// # Errors
/// Returns an error if the underlying query handler encounters an error.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn query(_ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::VerifyMembership(_) => todo!(),
        QueryMsg::VerifyNonMembership(_) => todo!(),
        QueryMsg::Status(_) => todo!(),
        QueryMsg::TimestampAtHeight(_) => todo!(),
    }
}
