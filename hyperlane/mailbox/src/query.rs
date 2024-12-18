use {
    crate::{CONFIG, DELIVERIES, NONCE},
    grug::{ImmutableCtx, Json, JsonSerExt, StdResult},
    hyperlane_types::mailbox::{Config, MessageId, QueryMsg},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Nonce {} => {
            let res = query_nonce(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Delivered { message_id } => {
            let res = query_delivered(ctx, message_id);
            res.to_json_value()
        },
    }
}

#[inline]
fn query_config(ctx: ImmutableCtx) -> StdResult<Config> {
    CONFIG.load(ctx.storage)
}

#[inline]
fn query_nonce(ctx: ImmutableCtx) -> StdResult<u32> {
    NONCE.current(ctx.storage)
}

#[inline]
fn query_delivered(ctx: ImmutableCtx, message_id: MessageId) -> bool {
    DELIVERIES.has(ctx.storage, message_id)
}
