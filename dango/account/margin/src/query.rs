use {
    crate::core,
    dango_auth::query_seen_nonces,
    dango_oracle::OracleQuerier,
    dango_types::{DangoQuerier, account::margin::QueryMsg},
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
            let app_cfg = ctx.querier.query_dango_config()?;
            let res = core::query_health(ctx.querier, ctx.contract, &app_cfg)?;
            res.to_json_value()
        },
        QueryMsg::Health { skip_if_no_debt } => {
            let oracle = ctx.querier.query_oracle()?;
            let mut oracle_querier = OracleQuerier::new_remote(oracle, ctx.querier);
            let res = core::query_and_compute_health(
                ctx.querier,
                &mut oracle_querier,
                ctx.contract,
                ctx.block.timestamp,
                None,
                skip_if_no_debt,
            )?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}
