use {
    dango_types::oracle::{InstantiateMsg, QueryMsg},
    grug::{ImmutableCtx, Json, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(_ctx: ImmutableCtx, _msg: QueryMsg) -> StdResult<Json> {
    todo!()
}
