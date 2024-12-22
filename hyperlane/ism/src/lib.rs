use {
    grug::{Empty, HexBinary, ImmutableCtx, Json, JsonSerExt, MutableCtx, Response, StdResult},
    hyperlane_types::ism::QueryMsg,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, query: QueryMsg) -> StdResult<Json> {
    match query {
        QueryMsg::Verify {
            raw_message,
            metadata,
        } => {
            query_verify(ctx, raw_message, metadata)?;
            ().to_json_value()
        },
    }
}

#[inline]
fn query_verify(
    _ctx: ImmutableCtx,
    _raw_message: HexBinary,
    _metadata: HexBinary,
) -> StdResult<()> {
    // TODO

    Ok(())
}
