use {
    dango_types::lending_pool::QueryMsg,
    grug::{ImmutableCtx, Json, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(_ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {}
}
