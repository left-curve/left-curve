use {
    crate::core,
    dango_auth::query_seen_nonces,
    dango_types::account::margin::QueryMsg,
    grug::{ImmutableCtx, Json, JsonSerExt},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::HealthData {} => {
            let res = core::query_health(&ctx.querier, ctx.contract, ctx.block.timestamp)?;
            res.to_json_value()
        },
        QueryMsg::Health {} => {
            let res = core::query_and_compute_health(
                &ctx.querier,
                ctx.contract,
                ctx.block.timestamp,
                None,
            )?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}
