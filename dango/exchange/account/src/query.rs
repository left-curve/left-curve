use {
    dango_auth::{query_seen_nonces, query_session_seen_nonces, query_status},
    dango_primitives::{ImmutableCtx, Json, JsonSerExt, StdResult},
    dango_types::account::QueryMsg,
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
