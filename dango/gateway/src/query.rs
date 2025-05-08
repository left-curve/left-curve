use {
    crate::{REVERSE_ROUTES, ROUTES},
    dango_types::gateway::{QueryMsg, Remote},
    grug::{Addr, Denom, ImmutableCtx, Json, JsonSerExt, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Route { bridge, remote } => {
            let res = query_route(ctx, bridge, remote)?;
            res.to_json_value()
        },
        QueryMsg::ReverseRoute { denom, remote } => {
            let res = query_reverse_route(ctx, denom, remote)?;
            res.to_json_value()
        },
    }
}

fn query_route(ctx: ImmutableCtx, bridge: Addr, remote: Remote) -> StdResult<Option<Denom>> {
    ROUTES.may_load(ctx.storage, (bridge, remote))
}

fn query_reverse_route(ctx: ImmutableCtx, denom: Denom, remote: Remote) -> StdResult<Option<Addr>> {
    REVERSE_ROUTES.may_load(ctx.storage, (&denom, remote))
}
