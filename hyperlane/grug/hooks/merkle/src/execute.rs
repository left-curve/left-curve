use {
    crate::{MAILBOX, MERKLE_TREE},
    anyhow::ensure,
    grug::{Hash256, HexBinary, MutableCtx, Response, StdResult},
    hyperlane_types::{
        hooks::{
            merkle::{ExecuteMsg, InsertedIntoTree, InstantiateMsg, PostDispatch},
            HookMsg,
        },
        IncrementalMerkleTree,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;
    MERKLE_TREE.save(ctx.storage, &IncrementalMerkleTree::default())?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Hook(HookMsg::PostDispatch { raw_message, .. }) => {
            post_dispatch(ctx, raw_message)
        },
    }
}

#[inline]
fn post_dispatch(ctx: MutableCtx, raw_message: HexBinary) -> anyhow::Result<Response> {
    // In the reference implementation, we should check here that the message ID
    // matches the mailbox's last dispatched ID.
    // Here instead, we just ensure the sender is the mailbox, and trust the
    // mailbox is properly implemented, i.e. it only calls this right after
    // dispatching a message.
    ensure!(
        ctx.sender == MAILBOX.load(ctx.storage)?,
        "sender is not mailbox"
    );

    // TODO: why don't we have the mailbox provide the `message_id`, so no need
    // to recompute here?
    let message_id = Hash256::from_inner(ctx.api.keccak256(&raw_message));

    let tree = MERKLE_TREE.update(ctx.storage, |mut tree| -> anyhow::Result<_> {
        tree.insert(message_id)?;
        Ok(tree)
    })?;

    Ok(Response::new()
        .add_event("post_dispatch", PostDispatch {
            message_id,
            index: tree.count - 1,
        })?
        .add_event("inserted_into_tree", InsertedIntoTree {
            index: tree.count - 1,
        })?)
}
