use {
    crate::{MAILBOX, REVERSE_ROUTES, ROUTES},
    anyhow::{bail, ensure},
    dango_types::{
        config::AppConfig,
        token_minter::{self, DestinationAddr, DestinationChain, HookTransferRemote},
        warp::{ExecuteMsg, Handle, InstantiateMsg, Route, TokenMessage},
    },
    grug::{Coin, Coins, Denom, HexBinary, Message, MutableCtx, QuerierExt, Response, StdResult},
    hyperlane_types::{
        Addr32,
        mailbox::{self, Domain},
        recipients::RecipientMsg,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetRoute {
            denom,
            destination_domain,
            route,
        } => set_route(ctx, denom, destination_domain, route),
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
        ExecuteMsg::HookTransferRemote(HookTransferRemote {
            token,
            destination_chain,
            recipient,
        }) => hook_transfer_remote(ctx, destination_chain, recipient, token),
    }
}

#[inline]
fn set_route(
    ctx: MutableCtx,
    denom: Denom,
    destination_domain: Domain,
    route: Route,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_route`"
    );

    ROUTES.save(ctx.storage, (&denom, destination_domain), &route)?;
    REVERSE_ROUTES.save(ctx.storage, (destination_domain, route.address), &denom)?;

    Ok(Response::new())
}

fn hook_transfer_remote(
    ctx: MutableCtx,
    chain: DestinationChain,
    recipient: DestinationAddr,
    token: Coin,
) -> anyhow::Result<Response> {
    let (
        DestinationChain::Hyperlane {
            domain: destination_domain,
        },
        DestinationAddr::Hyperlane(recipient),
    ) = (chain, recipient)
    else {
        bail!("only Hyperlane types are supported");
    };

    let route = ROUTES.load(ctx.storage, (&token.denom, destination_domain))?;

    Ok(Response::new().add_message(Message::execute(
        MAILBOX.load(ctx.storage)?,
        &mailbox::ExecuteMsg::Dispatch {
            destination_domain,
            // Note, this is the message recipient, not the token recipient.
            recipient: route.address,
            body: TokenMessage {
                recipient,
                amount: token.amount,
                metadata: Default::default(),
            }
            .encode(),
        },
        Coins::new(),
    )?))
}

// TODO: handle any the error that can happen here
#[inline]
fn handle(
    ctx: MutableCtx,
    origin_domain: Domain,
    sender: Addr32,
    body: HexBinary,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == MAILBOX.load(ctx.storage)?,
        "only mailbox can call `handle`"
    );

    // Deserialize the message.
    let body = TokenMessage::decode(&body)?;
    let denom = REVERSE_ROUTES.load(ctx.storage, (origin_domain, sender))?;
    let token_minter = ctx
        .querier
        .query_app_config::<AppConfig>()?
        .addresses
        .token_minter;

    Ok(Response::new()
        .add_message(Message::execute(
            token_minter,
            &token_minter::ExecuteMsg::ReceiveRemote {
                token: Coin::new(denom.clone(), body.amount)?,
                recipient: body.recipient.try_into()?,
            },
            Coins::default(),
        )?)
        .add_event(Handle {
            recipient: body.recipient,
            token: denom,
            amount: body.amount,
        })?)
}
