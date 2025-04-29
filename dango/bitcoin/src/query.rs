use {
    dango_types::bitcoin::QueryMsg,
    grug::{ImmutableCtx, Json, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(_ctx: ImmutableCtx, _msg: QueryMsg) -> StdResult<Json> {
    todo!()
}
