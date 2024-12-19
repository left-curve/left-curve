use {
    crate::{MAILBOX, MERKLE_TREE},
    anyhow::ensure,
    grug::{Hash256, HexBinary, MutableCtx, Response, StdResult},
    hyperlane_types::{
        merkle::{ExecuteMsg, InsertedIntoTree, InstantiateMsg, PostDispatch},
        merkle_tree::MerkleTree,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;
    MERKLE_TREE.save(ctx.storage, &MerkleTree::default())?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::PostDispatch { raw_message, .. } => post_dispatch(ctx, raw_message),
    }
}

#[inline]
fn post_dispatch(ctx: MutableCtx, raw_message: HexBinary) -> anyhow::Result<Response> {
    // This method can only be called by the mailbox contract, and the message
    // must match the last dispatched message ID.
    // Here we only check the sender, trusting the mailbox is implemented
    // correctly, i.e. will only call this right after a message is dispatched.
    ensure!(
        ctx.sender == MAILBOX.load(ctx.storage)?,
        "sender is not mailbox"
    );

    let message_id = Hash256::from_inner(ctx.api.keccak256(&raw_message));

    let tree = MERKLE_TREE.update(ctx.storage, |mut tree| -> anyhow::Result<_> {
        tree.insert(message_id)?;
        Ok(tree)
    })?;

    let index = tree.count - 1;

    Ok(Response::new()
        .add_event("post_dispatch", PostDispatch { message_id, index })?
        .add_event("inserted_into_tree", InsertedIntoTree { index })?)
}
