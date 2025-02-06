use {
    crate::{ALLOYS, MAILBOX, OUTBOUND_QUOTAS, RATE_LIMIT, REVERSE_ALLOYS, REVERSE_ROUTES, ROUTES},
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        warp::{
            ExecuteMsg, Handle, InstantiateMsg, RateLimit, Route, TokenMessage, TransferRemote,
            ALLOY_SUBNAMESPACE, NAMESPACE,
        },
    },
    grug::{
        Addr, Coin, Coins, Denom, HexBinary, Inner, IsZero, Message, MultiplyFraction, MutableCtx,
        Number, QuerierExt, QuerierWrapper, Response, StdResult, Storage, SudoCtx,
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
            rate_limit,
        } => set_route(ctx, denom, destination_domain, route, rate_limit),
        ExecuteMsg::SetAlloy {
            underlying_denom,
            alloyed_denom,
            destination_domain,
        } => set_alloy(ctx, underlying_denom, destination_domain, alloyed_denom),

        ExecuteMsg::SetRateLimit { denom, rate_limit } => set_rate_limit(
            ctx.storage,
            &ctx.querier,
            Some(ctx.sender),
            denom,
            rate_limit,
        ),
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    for (denom, rate_limit) in RATE_LIMIT
        .range(ctx.storage, None, None, grug::Order::Ascending)
        // Need to collect here because the iterator lock the ctx.storage.
        .collect::<StdResult<Vec<_>>>()?
    {
        update_outbound_quota(ctx.storage, &ctx.querier, denom, rate_limit)?;
    }

    Ok(Response::new())
}

#[inline]
fn set_route(
    ctx: MutableCtx,
    denom: Denom,
    destination_domain: Domain,
    route: Route,
    rate_limit: Option<RateLimit>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only chain owner can call `set_route`"
    );

    ROUTES.save(ctx.storage, (&denom, destination_domain), &route)?;
    REVERSE_ROUTES.save(ctx.storage, (destination_domain, route.address), &denom)?;

    if let Some(rate_limit) = rate_limit {
        set_rate_limit(ctx.storage, &ctx.querier, None, denom, rate_limit)
    } else {
        Ok(Response::new())
    }
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
fn set_rate_limit(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    sender: Option<Addr>,
    denom: Denom,
    rate_limit: RateLimit,
) -> anyhow::Result<Response> {
    if let Some(owner) = sender {
        ensure!(
            owner == querier.query_owner()?,
            "only chain owner can call `set_rate_limit`"
        );
    }

    RATE_LIMIT.save(storage, &denom, &rate_limit)?;

    update_outbound_quota(storage, querier, denom, rate_limit)
}

#[inline]

fn update_outbound_quota(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    denom: Denom,
    rate_limit: RateLimit,
) -> anyhow::Result<Response> {
    let limit = querier
        .query_supply(denom.clone())?
        .checked_mul_dec_floor(*rate_limit.inner())?;

    OUTBOUND_QUOTAS.save(storage, &denom, &limit)?;

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

    // Check if the token is alloyed.
    let (mut token, burn_alloy_msg) = if let Some(base_denom) =
        REVERSE_ALLOYS.may_load(ctx.storage, (&token.denom, destination_domain))?
    {
        // Burn the alloy token.
        let burn_alloy_msg = Message::execute(
            bank,
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
    if let Some(mut remaining) = OUTBOUND_QUOTAS.may_load(ctx.storage, &token.denom)? {
        remaining
            .checked_sub_assign(token.amount)
            .map_err(|_| anyhow!("rate limit reached: {} < {}", remaining, token.amount))?;

        OUTBOUND_QUOTAS.save(ctx.storage, &token.denom, &remaining)?;
    }

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
        .may_add_message(burn_alloy_msg)
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

    // Increase the rate limit remaining.
    if let Some(mut remaining) = OUTBOUND_QUOTAS.may_load(ctx.storage, &denom)? {
        remaining.checked_add_assign(body.amount)?;

        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &remaining)?;
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
