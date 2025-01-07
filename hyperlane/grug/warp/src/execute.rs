use {
    crate::{FEES, MAILBOX, REVERSE_ROUTES, ROUTES},
    anyhow::{anyhow, ensure},
    dango_types::bank,
    grug::{
        Coin, Coins, Denom, HexBinary, Message, MutableCtx, Number, Response, StdResult, Uint128,
    },
    hyperlane_types::{
        mailbox::{self, Domain},
        warp::{ExecuteMsg, Handle, InstantiateMsg, TokenMessage, TransferRemote, NAMESPACE},
        Addr32,
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
        ExecuteMsg::TransferRemote {
            destination_domain,
            recipient,
            metadata,
        } => transfer_remote(ctx, destination_domain, recipient, metadata),
        ExecuteMsg::SetRoute {
            denom,
            destination_domain,
            route,
        } => set_route(ctx, denom, destination_domain, route),
        ExecuteMsg::SetFee {
            denom,
            withdrawl_fee,
        } => set_fee(ctx, denom, withdrawl_fee),
        ExecuteMsg::Handle {
            origin_domain,
            sender,
            body,
        } => handle(ctx, origin_domain, sender, body),
    }
}

#[inline]
fn set_route(
    ctx: MutableCtx,
    denom: Denom,
    destination_domain: Domain,
    route: Addr32,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_route`"
    );

    ROUTES.save(ctx.storage, (&denom, destination_domain), &route)?;
    REVERSE_ROUTES.save(ctx.storage, (destination_domain, route), &denom)?;

    Ok(Response::new())
}

#[inline]
fn set_fee(ctx: MutableCtx, denom: Denom, withdrawal_fee: Uint128) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_fee`"
    );

    FEES.save(ctx.storage, &denom, &withdrawal_fee)?;

    Ok(Response::new())
}

#[inline]
fn transfer_remote(
    ctx: MutableCtx,
    destination_domain: Domain,
    recipient: Addr32,
    metadata: Option<HexBinary>,
) -> anyhow::Result<Response> {
    // Sender must attach exactly one token.
    let mut token = ctx.funds.into_one_coin()?;

    // The token must have a route and withdrawal fee set.
    let route = ROUTES.load(ctx.storage, (&token.denom, destination_domain))?;
    let fee = FEES.load(ctx.storage, &token.denom)?;

    token.amount.checked_sub_assign(fee).map_err(|_| {
        anyhow!(
            "withdrawal amount not sufficient to cover fee: {} < {}",
            token.amount,
            fee
        )
    })?;

    Ok(Response::new()
        // If the token is collateral, then escrow it (no need to do anything).
        // If it's synthetic, burn it.
        // We determine whether it's synthetic by checking whether its denom is
        // under the `hyp` namespace.
        .may_add_message(if token.denom.namespace() == Some(&NAMESPACE) {
            let bank = ctx.querier.query_bank()?;
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    denom: token.denom.clone(),
                    amount: token.amount,
                },
                Coins::new(),
            )?)
        } else {
            None
        })
        .add_message(Message::execute(
            MAILBOX.load(ctx.storage)?,
            &mailbox::ExecuteMsg::Dispatch {
                destination_domain,
                // Note, this is the message recipient, not the token recipient.
                recipient: route,
                body: TokenMessage {
                    recipient,
                    amount: token.amount,
                    metadata: metadata.unwrap_or_default(),
                }
                .encode(),
                // We currently don't support specifying custom hook and hook metadata.
                metadata: None,
                hook: None,
            },
            Coins::one(token.denom.clone(), fee)?,
        )?)
        .add_event("transfer_remote", &TransferRemote {
            sender: ctx.sender,
            destination_domain,
            recipient,
            token: token.denom,
            amount: token.amount,
            hook: None,
            metadata: None,
        })?)
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

    Ok(Response::new()
        // If the denom is synthetic, then mint the token.
        // Otherwise, if it's a collateral, then release the collateral.
        .add_message(if denom.namespace() == Some(&NAMESPACE) {
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: body.recipient.try_into()?,
                    denom: denom.clone(),
                    amount: body.amount,
                },
                Coins::new(),
            )?
        } else {
            // TODO: check whether the recipient exists; if not, register it at account factory.
            Message::transfer(body.recipient.try_into()?, Coin {
                denom: denom.clone(),
                amount: body.amount,
            })?
        })
        .add_event("handle", &Handle {
            recipient: body.recipient,
            token: denom,
            amount: body.amount,
        })?)
}
