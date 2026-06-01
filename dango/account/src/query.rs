use {
    dango_auth::{query_seen_nonces, query_session_seen_nonces, query_status},
    dango_types::account::QueryMsg,
    grug_types::{ImmutableCtx, Json, JsonSerExt, StdResult},
};

pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Status {} => {
            let res = query_status(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::SeenNonces {} => {
            let res = query_seen_nonces(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::SessionSeenNonces { session_key } => {
            let res = query_session_seen_nonces(ctx.storage, session_key)?;
            res.to_json_value()
        },
    }
}
