use {
    crate::{
        ALLOYS, MAILBOX, OUTBOUND_QUOTAS, RATE_LIMITS, REVERSE_ALLOYS, REVERSE_ROUTES, ROUTES,
    },
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        taxman::{self, FeeType},
        warp::{
            ExecuteMsg, Handle, InstantiateMsg, RateLimit, Route, TokenMessage, TransferRemote,
            ALLOY_SUBNAMESPACE, NAMESPACE,
        },
    },
    grug::{
        coins, Coin, Coins, Denom, HexBinary, Inner, IsZero, Message, MultiplyFraction, MutableCtx,
        Number, QuerierExt, Response, StdResult, SudoCtx,
    },
    hyperlane_types::{
        mailbox::{self, Domain},
        recipients::RecipientMsg,
        Addr32,
    },
    std::collections::BTreeMap,
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
        ExecuteMsg::SetAlloy {
            underlying_denom,
            alloyed_denom,
            destination_domain,
        } => set_alloy(ctx, underlying_denom, destination_domain, alloyed_denom),
        ExecuteMsg::SetRateLimits(limits) => set_rate_limits(ctx, limits),
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
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
fn set_alloy(
    ctx: MutableCtx,
    underlying_denom: Denom,
    destination_domain: Domain,
    alloyed_denom: Denom,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_alloy`"
    );

    ensure!(
        alloyed_denom.starts_with(&[NAMESPACE.clone(), ALLOY_SUBNAMESPACE.clone()]),
        "alloyed denom must start with `{}/{}`",
        NAMESPACE.as_ref(),
        ALLOY_SUBNAMESPACE.as_ref()
    );

    ALLOYS.save(ctx.storage, &underlying_denom, &alloyed_denom)?;

    REVERSE_ALLOYS.save(
        ctx.storage,
        (&alloyed_denom, destination_domain),
        &underlying_denom,
    )?;

    Ok(Response::new())
}

#[inline]
fn set_rate_limits(
    ctx: MutableCtx,
    limits: BTreeMap<Denom, RateLimit>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_rate_limits`"
    );

    RATE_LIMITS.save(ctx.storage, &limits)?;

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
    let cfg = ctx.querier.query_config()?;
    let token = ctx.funds.into_one_coin()?;

    // Check if the token is alloyed.
    let (mut token, burn_alloy_msg) = if let Some(base_denom) =
        REVERSE_ALLOYS.may_load(ctx.storage, (&token.denom, destination_domain))?
    {
        // Burn the alloy token.
        let burn_alloy_msg = Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom: token.denom.clone(),
                amount: token.amount,
            },
            Coins::new(),
        )?;

        (Coin::new(base_denom, token.amount)?, Some(burn_alloy_msg))
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

    // Check if the rate limit is reached.
    if let Some(mut quota) = OUTBOUND_QUOTAS.may_load(ctx.storage, &token.denom)? {
        quota.checked_sub_assign(token.amount).map_err(|_| {
            anyhow!(
                "withdrawal rate limit reached: {} < {}",
                quota,
                token.amount
            )
        })?;

        OUTBOUND_QUOTAS.save(ctx.storage, &token.denom, &quota)?;
    }

    // 1. Burn the alloy token, if the token being sent is an alloy token.
    // 2. If the token is collateral, escrow it (no need to do anything);
    //    otherwise (it's "synthetic", in Hyperlane's terminology), burn it.
    //    We determine whether it's synthetic by checking whether its denom is
    //    under the `hyp` namespace.
    // 3. Pay withdrawal fee to the taxman.
    // 4. Dispatch the Hyperlane message at the mailbox.
    Ok(Response::new()
        .may_add_message(burn_alloy_msg)
        .may_add_message(if token.denom.namespace() == Some(&NAMESPACE) {
            Some(Message::execute(
                cfg.bank,
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
        .may_add_message(if route.fee.is_non_zero() {
            Some(Message::execute(
                cfg.taxman,
                &taxman::ExecuteMsg::Pay {
                    user: ctx.sender,
                    ty: FeeType::Withdraw,
                },
                coins! { token.denom.clone() => route.fee },
            )?)
        } else {
            None
        })
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
            },
            Coins::new(),
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

    // Check if the denom is alloyed.
    let (denom, mint_underlying_msg) =
        if let Some(alloy_denom) = ALLOYS.may_load(ctx.storage, &denom)? {
            // Mint the base denom to the wrapper.
            let msg = Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    denom,
                    amount: body.amount,
                },
                Coins::new(),
            )?;
            (alloy_denom, Some(msg))
        } else {
            (denom, None)
        };

    // Increase the remaining outbound quota.
    if let Some(mut quota) = OUTBOUND_QUOTAS.may_load(ctx.storage, &denom)? {
        quota.checked_add_assign(body.amount)?;

        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &quota)?;
    }

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
            Message::transfer(body.recipient.try_into()?, Coin {
                denom: denom.clone(),
                amount: body.amount,
            })?
        })
        .may_add_message(mint_underlying_msg)
        .add_event(Handle {
            recipient: body.recipient,
            token: denom,
            amount: body.amount,
        })?)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    // Clear the quotas for the previous 24-hour window.
    OUTBOUND_QUOTAS.clear(ctx.storage, None, None);

    // Set quotes for the next 24-hour window.
    for (denom, limit) in RATE_LIMITS.load(ctx.storage)? {
        let supply = ctx.querier.query_supply(denom.clone())?;
        let quota = supply.checked_mul_dec_floor(limit.into_inner())?;
        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &quota)?;
    }

    Ok(Response::new())
}
