use {
    crate::{
        OUTBOUND_QUOTAS, PERSONAL_QUOTAS, RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES,
        WITHDRAWAL_FEES,
    },
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, Origin, PersonalQuota, RateLimit,
            Remote, SetPersonalQuotaRequest, Traceable, WithdrawalFee,
            bridge::{self, BridgeMsg},
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, Denom, Inner, IsZero, Message, MultiplyFraction, MutableCtx, Number,
        NumberConst, Op, QuerierExt, QuerierWrapper, Response, StdError, StdResult, Storage,
        SudoCtx, Uint128, btree_map, coins,
    },
    std::collections::{BTreeMap, BTreeSet},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    _set_routes(ctx.storage, msg.routes)?;
    _set_rate_limits(ctx.storage, msg.rate_limits)?;
    _set_withdrawal_fees(ctx.storage, msg.withdrawal_fees)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetRoutes(mapping) => set_routes(ctx, mapping),
        ExecuteMsg::SetRateLimits(rate_limits) => set_rate_limits(ctx, rate_limits),
        ExecuteMsg::SetWithdrawalFees(withdrawal_fees) => set_withdrawal_fees(ctx, withdrawal_fees),
        ExecuteMsg::ReceiveRemote {
            remote,
            amount,
            recipient,
        } => receive_remote(ctx, remote, amount, recipient),
        ExecuteMsg::TransferRemote { remote, recipient } => transfer_remote(ctx, remote, recipient),
        ExecuteMsg::SetPersonalQuota { user, denom, quota } => {
            set_personal_quota(ctx, user, denom, quota)
        },
    }
}

fn set_routes(
    ctx: MutableCtx,
    routes: BTreeSet<(Origin, Addr, Remote)>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set routes"
    );

    _set_routes(ctx.storage, routes)?;

    Ok(Response::new())
}

fn _set_routes(
    storage: &mut dyn Storage,
    routes: BTreeSet<(Origin, Addr, Remote)>,
) -> anyhow::Result<()> {
    for (origin, bridge, remote) in routes {
        let denom = match origin {
            Origin::Local(denom) => {
                ensure!(
                    !denom.is_remote(),
                    "local denom must not start with `{}` namespace: `{}`",
                    NAMESPACE.as_ref(),
                    denom
                );

                denom
            },
            Origin::Remote(part) => Denom::from_parts([NAMESPACE.clone(), part])?,
        };

        ROUTES.save(storage, (bridge, remote), &denom)?;
        REVERSE_ROUTES.save(storage, (&denom, remote), &bridge)?;
    }

    Ok(())
}

fn set_rate_limits(
    ctx: MutableCtx,
    rate_limits: BTreeMap<Denom, RateLimit>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set rate limits"
    );

    let old_rate_limits = RATE_LIMITS.load(ctx.storage)?;

    _set_rate_limits(ctx.storage, rate_limits.clone())?;
    tighten_quotas(ctx.storage, ctx.querier, &old_rate_limits, &rate_limits)?;

    Ok(Response::new())
}

fn _set_rate_limits(
    storage: &mut dyn Storage,
    rate_limits: BTreeMap<Denom, RateLimit>,
) -> StdResult<()> {
    RATE_LIMITS.save(storage, &rate_limits)?;

    Ok(())
}

/// Clear all outbound quotas and reseed them from each rate-limited denom's
/// current supply times its configured percentage. Called by the cron job at
/// the start of each 24-hour window.
fn reseed_quotas(storage: &mut dyn Storage, querier: QuerierWrapper) -> StdResult<()> {
    OUTBOUND_QUOTAS.clear(storage, None, None);

    for (denom, limit) in RATE_LIMITS.load(storage)? {
        let supply = querier.query_supply(denom.clone())?;
        let quota = supply.checked_mul_dec_floor(limit.into_inner())?;

        OUTBOUND_QUOTAS.save(storage, &denom, &quota)?;
    }

    Ok(())
}

/// Called by `set_rate_limits` to reconcile outstanding quotas with the new
/// rate-limits map. `SetRateLimits` only ever lowers outstanding quotas;
/// raises wait for the next cron tick. This prevents an admin from refilling
/// a drained quota mid-window (intentionally or accidentally) and letting the
/// same user withdraw a second full window back-to-back.
///
/// - Newly added denoms are seeded at `supply × limit`.
/// - Existing denoms take the smaller of the current quota and `supply × new_limit`.
/// - Denoms removed from the map have their quota entry dropped (become
///   unrestricted immediately).
fn tighten_quotas(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    old_rate_limits: &BTreeMap<Denom, RateLimit>,
    new_rate_limits: &BTreeMap<Denom, RateLimit>,
) -> StdResult<()> {
    for denom in old_rate_limits.keys() {
        if !new_rate_limits.contains_key(denom) {
            OUTBOUND_QUOTAS.remove(storage, denom);
        }
    }

    for (denom, limit) in new_rate_limits {
        let supply = querier.query_supply(denom.clone())?;
        let fresh_quota = supply.checked_mul_dec_floor(limit.into_inner())?;

        let tightened = OUTBOUND_QUOTAS
            .may_load(storage, denom)?
            .map_or(fresh_quota, |current| current.min(fresh_quota));

        OUTBOUND_QUOTAS.save(storage, denom, &tightened)?;
    }

    Ok(())
}

fn set_withdrawal_fees(
    ctx: MutableCtx,
    withdrawal_fees: Vec<WithdrawalFee>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set withdrawal fees"
    );

    _set_withdrawal_fees(ctx.storage, withdrawal_fees)?;

    Ok(Response::new())
}

fn _set_withdrawal_fees(
    storage: &mut dyn Storage,
    withdrawal_fees: Vec<WithdrawalFee>,
) -> StdResult<()> {
    for WithdrawalFee { denom, remote, fee } in withdrawal_fees {
        match fee {
            Op::Insert(fee) => {
                WITHDRAWAL_FEES.save(storage, (&denom, remote), &fee)?;
            },
            Op::Delete => {
                WITHDRAWAL_FEES.remove(storage, (&denom, remote));
            },
        }
    }

    Ok(())
}

fn set_personal_quota(
    ctx: MutableCtx,
    user: Addr,
    denom: Denom,
    quota: Op<SetPersonalQuotaRequest>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set personal quotas"
    );

    match quota {
        Op::Insert(SetPersonalQuotaRequest {
            amount,
            available_for,
        }) => {
            let expiry = available_for
                .map(|d| ctx.block.timestamp.checked_add(d))
                .transpose()?;

            PERSONAL_QUOTAS.save(ctx.storage, (user, &denom), &PersonalQuota {
                amount,
                expiry,
            })?;
        },
        Op::Delete => {
            PERSONAL_QUOTAS.remove(ctx.storage, (user, &denom));
        },
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

    // Increase the reserve only if the denom is remote.
    if denom.is_remote() {
        RESERVES.may_update(ctx.storage, (ctx.sender, remote), |maybe_reserve| {
            let reserve = maybe_reserve.unwrap_or(Uint128::ZERO);

            Ok::<_, StdError>(reserve.checked_add(amount)?)
        })?;
    }

    // First,
    // - if the token is not native on Dango, mint it to the Gateway contract;
    // - otherwise, the token should already been in the Gateway contract, no need
    //   to mint.
    // Then, transfer the token from Gateway to the recipient.
    //
    // Why mint to Gateway first and then transfer to recipient, instead of
    // directly minting to recipient? Because minting doesn't trigger the recipient's
    // `receive` entry point, only transferring does. In some cases, we do need
    // `receive` to be triggered; e.g. activating a new account (see `dango_auth::receive_transfer`).
    Ok(Response::new()
        .may_add_message(if denom.is_remote() {
            let bank = ctx.querier.query_bank()?;
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.contract,
                    coins: coins! { denom.clone() => amount },
                },
                Coins::new(),
            )?)
        } else {
            None
        })
        .add_message(Message::transfer(recipient, coins! { denom => amount })?))
}

fn transfer_remote(ctx: MutableCtx, remote: Remote, recipient: Addr32) -> anyhow::Result<Response> {
    // The user must have sent exactly one coin.
    let mut coin = ctx.funds.into_one_coin()?;

    // Find the bridge contract corresponding to the (denom, remote) tuple.
    let bridge = REVERSE_ROUTES.load(ctx.storage, (&coin.denom, remote))?;

    // Deduct the withdrawal fee.
    let maybe_fee = WITHDRAWAL_FEES.may_load(ctx.storage, (&coin.denom, remote))?;

    if let Some(fee) = maybe_fee {
        coin.amount.checked_sub_assign(fee).map_err(|_| {
            anyhow!(
                "withdrawal amount not sufficient to cover fee: {} < {}",
                coin.amount,
                fee
            )
        })?;
    }

    // Reduce the reserve only if the denom is remote.
    if coin.denom.is_remote() {
        RESERVES.may_update(ctx.storage, (bridge, remote), |maybe_reserve| {
            let reserve = maybe_reserve.unwrap_or(Uint128::ZERO);
            reserve.checked_sub(coin.amount).map_err(|_| {
                anyhow!(
                    "insufficient reserve! bridge: {}, remote: {:?}, reserve: {}, amount: {}",
                    bridge,
                    remote,
                    reserve,
                    coin.amount
                )
            })
        })?;
    }

    // Consume the sender's personal quota first, if any and still active.
    // Whatever is left over falls through to the global outbound quota.
    let key = (ctx.sender, &coin.denom);
    let mut remaining = coin.amount;

    if let Some(pq) = PERSONAL_QUOTAS.may_load(ctx.storage, key)?
        && pq.expiry.is_none_or(|e| ctx.block.timestamp < e)
    {
        let consumed = pq.amount.min(remaining);
        remaining = remaining.checked_sub(consumed)?;

        let leftover = pq.amount.checked_sub(consumed)?;
        if leftover.is_zero() {
            PERSONAL_QUOTAS.remove(ctx.storage, key);
        } else {
            PERSONAL_QUOTAS.save(ctx.storage, key, &PersonalQuota {
                amount: leftover,
                expiry: pq.expiry,
            })?;
        }
    }

    // Reduce the global outbound quota by whatever the personal quota did not
    // cover. A missing entry means the denom is not rate-limited at all.
    if !remaining.is_zero() {
        OUTBOUND_QUOTAS.may_modify(ctx.storage, &coin.denom, |maybe_quota| {
            let Some(quota) = maybe_quota else {
                return Ok(None);
            };

            Some(quota.checked_sub(remaining).map_err(|_| {
                anyhow!(
                    "insufficient outbound quota! denom: {}, requested: {}, remaining after personal quota: {}",
                    coin.denom,
                    coin.amount,
                    remaining
                )
            }))
            .transpose()
        })?;
    }

    let (bank, taxman) = ctx.querier.query_bank_and_taxman()?;

    // 1. Call the bridge contract to make the remote transfer.
    // 2. Burn the alloyed token to be transferred (only if the token is not native on Dango).
    // 3. Pay fee to the taxman.
    Ok(Response::new()
        .add_message(Message::execute(
            bridge,
            &bridge::ExecuteMsg::Bridge(BridgeMsg::TransferRemote {
                remote,
                amount: coin.amount,
                recipient,
            }),
            Coins::new(),
        )?)
        .may_add_message(if coin.denom.is_remote() {
            Some(Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: coin.clone().into(),
                },
                Coins::new(),
            )?)
        } else {
            None
        })
        .may_add_message(if let Some(fee) = maybe_fee {
            Some(Message::execute(
                taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Withdraw,
                    payments: btree_map! {
                        ctx.sender => coins! { coin.denom.clone() => fee },
                    },
                },
                coins! { coin.denom => fee },
            )?)
        } else {
            None
        }))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    reseed_quotas(ctx.storage, ctx.querier)?;

    Ok(Response::new())
}
