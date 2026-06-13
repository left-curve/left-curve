use {
    crate::CONFIG,
    dango_types::taxman::{Config, QueryMsg},
    grug_types::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}
