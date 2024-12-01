//! Defines the query interface for the ics07-tendermint contract.

use anyhow::Result;
use dango_types::ibc_client::QueryMsg;
use grug::{ImmutableCtx, Json};

use crate::ctx::TendermintContext;

/// The query entrypoint for the contract.
/// # Errors
/// Returns an error if the underlying query handler encounters an error.
#[cfg_attr(not(feature = "library"), grug::export)]
#[allow(clippy::needless_pass_by_value)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> Result<Json> {
    TendermintContext::new_ref(ctx)?.query(msg)
}
