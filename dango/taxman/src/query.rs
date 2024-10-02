use {
    crate::CONFIG,
    dango_types::taxman::{Config, QueryMsg},
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => query_config(ctx)?.to_json_value(),
    }
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}
