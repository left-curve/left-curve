use {
    crate::{MAILBOX, OUTBOUND_QUOTAS, RATE_LIMITS, REVERSE_ROUTES, ROUTES},
    anyhow::{anyhow, ensure},
    dango_types::{
        DangoQuerier, bank, gateway,
        taxman::{self, FeeType},
        warp::{
            ExecuteMsg, Handle, InstantiateMsg, NAMESPACE, RateLimit, Route, TokenMessage,
            TransferRemote,
        },
    },
    grug::{
        Coin, Coins, Denom, HexBinary, Inner, IsZero, Message, MultiplyFraction, MutableCtx,
        Number, QuerierExt, Response, StdResult, SudoCtx, btree_map, coins,
    },
    hyperlane_types::{
        Addr32,
        mailbox::{self, Domain},
        recipients::RecipientMsg,
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

    // 1. If the token is collateral, escrow it (no need to do anything);
    //    otherwise (it's "synthetic", in Hyperlane's terminology), burn it.
    //    We determine whether it's synthetic by checking whether its denom is
    //    under the `hyp` namespace.
    // 2. Pay withdrawal fee to the taxman.
    // 3. Dispatch the Hyperlane message at the mailbox.
    Ok(Response::new()
        .may_add_message(if token.denom.namespace() == Some(&NAMESPACE) {
            Some(Message::execute(
                cfg.bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: coins! { token.denom.clone() => token.amount },
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
                    ty: FeeType::Withdraw,
                    payments: btree_map! {
                        ctx.sender => coins! { token.denom.clone() => route.fee },
                    },
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

    // Increase the remaining outbound quota.
    if let Some(mut quota) = OUTBOUND_QUOTAS.may_load(ctx.storage, &denom)? {
        quota.checked_add_assign(body.amount)?;

        OUTBOUND_QUOTAS.save(ctx.storage, &denom, &quota)?;
    }

    // If the denom is synthetic:
    // 1. Mint the underlying coins to self.
    // 2. Send the underlying coins to be alloyed and forwarded to the recipient.
    // Otherwise, it's a collateral, release the collateral.
    Ok(Response::new()
        .add_messages(if denom.namespace() == Some(&NAMESPACE) {
            let gateway = ctx.querier.query_gateway()?;
            let underlying_coins = coins! { denom.clone() => body.amount };
            vec![
                Message::execute(
                    bank,
                    &bank::ExecuteMsg::Mint {
                        to: ctx.contract,
                        coins: underlying_coins.clone(),
                    },
                    Coins::new(),
                )?,
                Message::execute(
                    gateway,
                    &gateway::ExecuteMsg::Alloy {
                        and_then: Some(gateway::Action::Transfer(body.recipient.try_into()?)),
                    },
                    underlying_coins,
                )?,
            ]
        } else {
            vec![Message::transfer(body.recipient.try_into()?, Coin {
                denom: denom.clone(),
                amount: body.amount,
            })?]
        })
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
