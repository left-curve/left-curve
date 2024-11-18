use {
    crate::MarginQuerier,
    dango_auth::NEXT_SEQUENCE,
    dango_types::account::margin::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Sequence {} => {
            let res = query_sequence(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Health {} => {
            let res = ctx.querier.query_health(ctx.contract)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_sequence(storage: &dyn Storage) -> StdResult<u32> {
    NEXT_SEQUENCE.current(storage)
}
