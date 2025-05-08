use {
    crate::MAILBOX,
    dango_types::warp::QueryMsg,
    grug::{Addr, ImmutableCtx, Json, JsonSerExt, StdResult},
    hyperlane_types::recipients::{RecipientQuery, RecipientQueryResponse},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Mailbox {} => {
            let res = query_mailbox(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Recipient(RecipientQuery::InterchainSecurityModule {}) => {
            let ism = query_interchain_security_module(ctx);
            let res = RecipientQueryResponse::InterchainSecurityModule(ism);
            res.to_json_value()
        },
    }
}

fn query_mailbox(ctx: ImmutableCtx) -> StdResult<Addr> {
    MAILBOX.load(ctx.storage)
}

#[inline]
fn query_interchain_security_module(_ctx: ImmutableCtx) -> Option<Addr> {
    // Currently we just use the default ISM.
    None
}
