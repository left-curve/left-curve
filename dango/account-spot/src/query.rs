use {
    dango_auth::NEXT_SEQUENCE,
    dango_types::account::spot::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Sequence {} => query_sequence(ctx.storage)?.to_json_value(),
    }
}

fn query_sequence(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_SEQUENCE.current(storage)
}
