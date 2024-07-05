#[cfg(not(feature = "library"))]
use grug_macros::grug_export;
use {
    crate::{
        authenticate_tx, initialize, query_state, update_key, ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    grug_types::{to_json_value, AuthCtx, ImmutableCtx, Json, MutableCtx, Response, StdResult, Tx},
};

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    initialize(ctx.storage, &msg.public_key)
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn before_tx(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    authenticate_tx(ctx, tx)
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn after_tx(_ctx: AuthCtx, _tx: Tx) -> StdResult<Response> {
    // Nothing to do
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateKey { new_public_key } => update_key(ctx, &new_public_key),
    }
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::State {} => to_json_value(&query_state(ctx.storage)?),
    }
}
