use {
    crate::{MAILBOX, MERKLE_TREE},
    grug::{Addr, Coins, ImmutableCtx, Json, JsonSerExt, StdResult},
    hyperlane_types::{
        hooks::{merkle::QueryMsg, HookQuery, HookQueryResponse},
        IncrementalMerkleTree,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Mailbox {} => {
            let res = query_mailbox(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Tree {} => {
            let res = query_tree(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Hook(HookQuery::QuoteDispatch { .. }) => {
            let res = HookQueryResponse::QuoteDispatch(quote_dispatch());
            res.to_json_value()
        },
    }
}

#[inline]
fn query_mailbox(ctx: ImmutableCtx) -> StdResult<Addr> {
    MAILBOX.load(ctx.storage)
}

#[inline]
fn query_tree(ctx: ImmutableCtx) -> StdResult<IncrementalMerkleTree> {
    MERKLE_TREE.load(ctx.storage)
}

#[inline]
fn quote_dispatch() -> Coins {
    Coins::new()
}
