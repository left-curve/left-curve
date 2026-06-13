use {
    crate::MAILBOX,
    anyhow::{bail, ensure},
    dango_hyperlane_types::{
        Addr32,
        mailbox::{self, Domain},
        recipients::RecipientMsg,
    },
    dango_math::Uint128,
    dango_primitives::{Coins, HexBinary, Message, MutableCtx, Response, StdResult},
    dango_types::{
        DangoQuerier,
        gateway::{self, Remote, bridge::BridgeMsg},
        warp::{ExecuteMsg, InstantiateMsg, TokenMessage},
    },
};

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    MAILBOX.save(ctx.storage, &msg.mailbox)?;

    Ok(Response::new())
}

pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
        ExecuteMsg::Bridge(BridgeMsg::TransferRemote {
            remote,
            amount,
            recipient,
        }) => transfer_remote(ctx, remote, amount, recipient),
    }
}

fn transfer_remote(
    ctx: MutableCtx,
    remote: Remote,
    amount: Uint128,
    recipient: Addr32,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_gateway()?,
        "only gateway can call `transfer_remote`"
    );

    let Remote::Warp { domain, contract } = remote else {
        bail!("incorrect remote type! expecting: warp, found: {remote:?}");
    };

    Ok(Response::new().add_message({
        let mailbox = MAILBOX.load(ctx.storage)?;
        Message::execute(
            mailbox,
            &mailbox::ExecuteMsg::Dispatch {
                destination_domain: domain,
                // Note, this is the message recipient, not the token recipient.
                recipient: contract,
                body: TokenMessage {
                    recipient,
                    amount,
                    // Metadata isn't supported at this time.
                    metadata: HexBinary::default(),
                }
                .encode(),
            },
            Coins::new(),
        )?
    }))
}

// TODO: handle any the error that can happen here
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

    let body = TokenMessage::decode(&body)?;

    Ok(Response::new().add_message({
        let gateway = ctx.querier.query_gateway()?;
        Message::execute(
            gateway,
            &gateway::ExecuteMsg::ReceiveRemote {
                remote: Remote::Warp {
                    domain: origin_domain,
                    contract: sender,
                },
                amount: body.amount,
                recipient: body.recipient.try_into()?,
            },
            Coins::new(),
        )?
    }))
}
