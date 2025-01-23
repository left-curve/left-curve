use {
    crate::{MAILBOX, RATE_LIMIT, REVERSE_ROUTES, ROUTES},
    anyhow::{anyhow, ensure},
    dango_types::bank,
    grug::{
        Addr, Coin, Coins, Denom, HexBinary, Inner, IsZero, Message, MultiplyFraction, MutableCtx,
        Number, QuerierExt, QuerierWrapper, Response, StdResult, Storage, SudoCtx,
    },
    hyperlane_types::{
        mailbox::{self, Domain},
        recipients::{
            warp::{
                ExecuteMsg, Handle, InstantiateMsg, RateLimit, RateLimitConfig, Route,
                TokenMessage, TransferRemote, NAMESPACE,
            },
            RecipientMsg,
        },
        Addr32,
    },
    std::cmp::max,
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
        ExecuteMsg::Recipient(RecipientMsg::Handle {
            origin_domain,
            sender,
            body,
        }) => handle(ctx, origin_domain, sender, body),
        ExecuteMsg::SetRateLimit { denom, rate_limit } => set_rate_limit(
            ctx.storage,
            &ctx.querier,
            Some(ctx.sender),
            denom,
            rate_limit,
        ),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    for (denom, rate_limit) in RATE_LIMIT
        .range(ctx.storage, None, None, grug::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        set_rate_limit(ctx.storage, &ctx.querier, None, denom, RateLimitConfig {
            min_remaining: rate_limit.min_remaining,
            supply_share: rate_limit.supply_share,
        })?;
    }

    Ok(Response::new())
}

#[inline]
fn set_route(
    ctx: MutableCtx,
    denom: Denom,
    destination_domain: Domain,
    route: Route,
    rate_limit: Option<RateLimitConfig>,
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

fn set_rate_limit(
    storage: &mut dyn Storage,
    querier: &QuerierWrapper,
    sender: Option<Addr>,
    denom: Denom,
    rate_limit: RateLimitConfig,
) -> anyhow::Result<Response> {
    if let Some(owner) = sender {
        ensure!(
            owner == querier.query_owner()?,
            "only chain owner can call `set_rate_limit`"
        );
    }

    let supply = querier.query_supply(denom.clone())?;

    let remaining = max(
        supply.checked_mul_dec_floor(*rate_limit.supply_share.inner())?,
        rate_limit.min_remaining,
    );

    RATE_LIMIT.save(storage, &denom, &RateLimit {
        remaining,
        min_remaining: rate_limit.min_remaining,
        supply_share: rate_limit.supply_share,
    })?;

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

    // The token must have a route set.
    let route = ROUTES.load(ctx.storage, (&token.denom, destination_domain))?;

    token.amount.checked_sub_assign(route.fee).map_err(|_| {
        anyhow!(
            "withdrawal amount not sufficient to cover fee: {} < {}",
            token.amount,
            route.fee
        )
    })?;

    if let Some(mut rate_limit) = RATE_LIMIT.may_load(ctx.storage, &token.denom)? {
        rate_limit
            .remaining
            .checked_sub_assign(token.amount)
            .map_err(|_| {
                anyhow!(
                    "rate limit reached: {} < {}",
                    rate_limit.remaining,
                    token.amount
                )
            })?;

        RATE_LIMIT.save(ctx.storage, &token.denom, &rate_limit)?;
    }

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

    if let Some(mut rate_limit) = RATE_LIMIT.may_load(ctx.storage, &denom)? {
        rate_limit.remaining.checked_add_assign(body.amount)?;

        RATE_LIMIT.save(ctx.storage, &denom, &rate_limit)?;
    }

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
