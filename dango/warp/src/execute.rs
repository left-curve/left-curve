use {
    crate::{ALLOYED, MAILBOX, REVERSE_ROUTES, ROUTES},
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        warp::{
            Alloyed, ExecuteMsg, Handle, InstantiateMsg, Route, TokenMessage, TransferRemote,
            NAMESPACE,
        },
    },
    grug::{
        Coin, Coins, Denom, HexBinary, IsZero, Message, MutableCtx, Number, QuerierExt, Response,
        StdResult,
    },
    hyperlane_types::{
        mailbox::{self, Domain},
        recipients::RecipientMsg,
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
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
        ExecuteMsg::RegisterAlloy {
            base_denom,
            alloyed_denom,
            destination_domain,
        } => register_alloy(ctx, base_denom, alloyed_denom, destination_domain),
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

#[inline]
fn transfer_remote(
    ctx: MutableCtx,
    destination_domain: Domain,
    recipient: Addr32,
    metadata: Option<HexBinary>,
) -> anyhow::Result<Response> {
    // Sender must attach exactly one token.
    let token = ctx.funds.into_one_coin()?;

    let bank = ctx.querier.query_bank()?;

    // Check if the token is alloyed
    let (mut token, brun_alloyed_msg) = if let Some((base_denom, _)) = ALLOYED
        .idx
        .alloyed_domain
        .may_load(ctx.storage, (token.denom.clone(), destination_domain))?
    {
        // Burn the alloy token
        let brun_alloyed_msg = Message::execute(
            bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom: token.denom.clone(),
                amount: token.amount,
            },
            Coins::new(),
        )?;

        let base_token = Coin::new(base_denom, token.amount)?;

        (base_token, Some(brun_alloyed_msg))
    } else {
        (token, None)
    };

    // The token must have a route set.
    let route = ROUTES.load(ctx.storage, (&token.denom, destination_domain))?;

    token.amount.checked_sub_assign(route.fee).map_err(|_| {
        anyhow!(
            "withdrawal amount not sufficient to cover fee: {} < {}",
            token.amount,
            route.fee
        )
    })?;

    Ok(Response::new()
        // If the token is collateral, then escrow it (no need to do anything).
        // If it's synthetic, burn it.
        // We determine whether it's synthetic by checking whether its denom is
        // under the `hyp` namespace.
        .may_add_message(if token.denom.namespace() == Some(&NAMESPACE) {
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
        .may_add_message(brun_alloyed_msg)
        .add_message(Message::execute(
            MAILBOX.load(ctx.storage)?,
            &mailbox::ExecuteMsg::Dispatch {
                destination_domain,
                // Note, this is the message recipient, not the token recipient.
                recipient: route.address,
                body: TokenMessage {
                    recipient,
                    amount: token.amount,
                    metadata: metadata.unwrap_or_default(),
                }
                .encode(),
                // For sending tokens, we currently don't support metadata.
                metadata: None,
                // Always use the mailbox's default hook, which is set to the
                // fee hook. This hook will get the withdrawal fee. We don't
                // want the user to specify a different hook and steal the fee.
                hook: None,
            },
            {
                if route.fee.is_zero() {
                    Coins::new()
                } else {
                    Coins::one(token.denom.clone(), route.fee)?
                }
            },
        )?)
        .add_event(TransferRemote {
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
    let bank = ctx.querier.query_bank()?;

    // Check if the denom is alloyed
    let (denom, mint_base_denom_msg) =
        if let Some(alloyed) = ALLOYED.may_load(ctx.storage, denom.clone())? {
            // Mint the base denom to the wrapper
            let mint_msg = Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    denom,
                    amount: body.amount,
                },
                Coins::new(),
            )?;
            (alloyed.alloyed_denom, Some(mint_msg))
        } else {
            (denom, None)
        };

    Ok(Response::new()
        // If the denom is synthetic, then mint the token.
        // Otherwise, if it's a collateral, then release the collateral.
        .add_message(if denom.namespace() == Some(&NAMESPACE) {
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
        .may_add_message(mint_base_denom_msg)
        .add_event(Handle {
            recipient: body.recipient,
            token: denom,
            amount: body.amount,
        })?)
}

fn register_alloy(
    ctx: MutableCtx,
    base_denom: Denom,
    alloyed_denom: Denom,
    destination_domain: Domain,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `register_alloy`"
    );

    ensure!(
        alloyed_denom.namespace() == Some(&NAMESPACE),
        "alloyed_denom must be in the `hyp` namespace"
    );

    let alloyed = Alloyed {
        alloyed_denom,
        destination_domain,
    };

    ALLOYED.save(ctx.storage, base_denom, &alloyed)?;

    Ok(Response::new())
}
