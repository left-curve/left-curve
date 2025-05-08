use {
    crate::{RESERVES, REVERSE_ROUTES, ROUTES},
    anyhow::ensure,
    dango_types::{
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, Remote,
            bridge::{self, BridgeMsg},
        },
    },
    grug::{
        Addr, Coins, Denom, Message, MutableCtx, Number, NumberConst, Part, QuerierExt, Response,
        StdError, StdResult, Uint128, coins,
    },
    std::collections::BTreeSet,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    _set_routes(ctx, msg.routes)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetRoutes(mapping) => set_routes(ctx, mapping),
        ExecuteMsg::ReceiveRemote {
            remote,
            amount,
            recipient,
        } => receive_remote(ctx, remote, amount, recipient),
        ExecuteMsg::TransferRemote { remote, recipient } => transfer_remote(ctx, remote, recipient),
    }
}

fn set_routes(ctx: MutableCtx, routes: BTreeSet<(Part, Addr, Remote)>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    _set_routes(ctx, routes)?;

    Ok(Response::new())
}

fn _set_routes(ctx: MutableCtx, routes: BTreeSet<(Part, Addr, Remote)>) -> StdResult<Response> {
    for (part, bridge, remote) in routes {
        let denom = Denom::from_parts([NAMESPACE.clone(), part])?;

        ROUTES.save(ctx.storage, (bridge, remote), &denom)?;
        REVERSE_ROUTES.save(ctx.storage, (&denom, remote), &bridge)?;
        RESERVES.save(ctx.storage, (bridge, remote), &Uint128::ZERO)?;
    }

    Ok(Response::new())
}

fn receive_remote(
    ctx: MutableCtx,
    remote: Remote,
    amount: Uint128,
    recipient: Addr,
) -> anyhow::Result<Response> {
    // Find the alloyed denom of the given bridge contract and remote.
    let denom = ROUTES.load(ctx.storage, (ctx.sender, remote))?;

    // Increase the reserve corresponding to the (bridge, remote) tuple.
    RESERVES.update(ctx.storage, (ctx.sender, remote), |reserve| {
        Ok::<_, StdError>(reserve.checked_add(amount)?)
    })?;

    // Mint the alloyed token to the recipient.
    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: recipient,
                coins: coins! { denom => amount },
            },
            Coins::new(),
        )?
    }))
}

fn transfer_remote(ctx: MutableCtx, remote: Remote, recipient: Addr32) -> anyhow::Result<Response> {
    // The user must have sent exactly one coin.
    let coin = ctx.funds.into_one_coin()?;

    // Find the bridge contract corresponding to the (denom, remote) tuple.
    let bridge = REVERSE_ROUTES.load(ctx.storage, (&coin.denom, remote))?;

    // Reduce the reserve corresponding to the (bridge, remote) tuple.
    RESERVES.update(ctx.storage, (bridge, remote), |reserve| -> StdResult<_> {
        Ok::<_, StdError>(reserve.checked_sub(coin.amount)?)
    })?;

    // Call the bridge contract to make the remote transfer.
    Ok(Response::new().add_message(Message::execute(
        bridge,
        &bridge::ExecuteMsg::Bridge(BridgeMsg::TransferRemote {
            remote,
            amount: coin.amount,
            recipient,
        }),
        Coins::new(),
    )?))
}
