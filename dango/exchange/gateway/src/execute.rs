use {
    crate::{PERSONAL_QUOTAS, RESERVES, REVERSE_ROUTES, ROUTES, WITHDRAWAL_FEES, rate_limit},
    anyhow::{anyhow, ensure},
    dango_math::{IsZero, Number, NumberConst, Uint128},
    dango_primitives::{
        Addr, Coins, Denom, Inner, Message, MutableCtx, Op, Order, QuerierExt, Response, StdError,
        StdResult, Storage, SudoCtx, coins,
    },
    dango_types::{
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, Origin, PersonalQuota, RateLimit,
            Remote, SetPersonalQuotaRequest, Traceable, WithdrawalFee,
            bridge::{self, BridgeMsg},
        },
    },
    std::collections::{BTreeMap, BTreeSet},
};

pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    _set_routes(ctx.storage, msg.routes)?;
    rate_limit::init(ctx.storage, msg.rate_limits)?;
    _set_withdrawal_fees(ctx.storage, msg.withdrawal_fees)?;

    Ok(Response::new())
}

pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SetRoutes(mapping) => set_routes(ctx, mapping),
        ExecuteMsg::RemoveRoutes(routes) => remove_routes(ctx, routes),
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

fn remove_routes(ctx: MutableCtx, routes: BTreeSet<(Addr, Remote)>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can remove routes"
    );

    for (bridge, remote) in routes {
        // Load the denom of the route. Errors if the route doesn't exist.
        let denom = ROUTES.load(ctx.storage, (bridge, remote))?;

        // The reserve of the route must be zero: either no entry exists (the
        // route was never funded, or its denom is local-origin, for which
        // reserves aren't tracked), or the entry has been drained to exactly
        // zero. Otherwise, removing the route would make it impossible for
        // the reserve to be withdrawn, as `transfer_remote` requires the
        // reverse route to exist.
        let reserve = RESERVES
            .may_load(ctx.storage, (bridge, remote))?
            .unwrap_or(Uint128::ZERO);

        ensure!(
            reserve.is_zero(),
            "can't remove route with non-zero reserve! bridge: {bridge}, remote: {remote:?}, reserve: {reserve}"
        );

        // Delete the route, its reverse mapping, and the (zero-valued)
        // reserve entry, so that reserve enumeration doesn't show dangling
        // zeros. If the route is re-added later, `receive_remote` recreates
        // the reserve entry upon the first inbound transfer.
        ROUTES.remove(ctx.storage, (bridge, remote));
        REVERSE_ROUTES.remove(ctx.storage, (&denom, remote));
        RESERVES.remove(ctx.storage, (bridge, remote));
    }

    Ok(Response::new())
}

fn set_rate_limits(
    ctx: MutableCtx,
    rate_limits: BTreeMap<Denom, RateLimit>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set rate limits"
    );

    // A 0% rate limit is a hard freeze: the global cap is zero, so it must
    // also revoke any personal quota that would otherwise let a user bypass
    // the freeze through their per-account allowance.
    //
    // Compute the frozen-denom set from the incoming map and run the
    // revocation pass before delegating to `rate_limit::apply_admin_update`.
    // Personal quotas are not rate-limit machinery and live outside the
    // `rate_limit` module; keeping the revocation here means that module
    // doesn't have to know about `PERSONAL_QUOTAS`.
    let frozen_denoms: BTreeSet<&Denom> = rate_limits
        .iter()
        .filter(|(_, limit)| limit.into_inner().is_zero())
        .map(|(denom, _)| denom)
        .collect();

    if !frozen_denoms.is_empty() {
        let personal_quotas = PERSONAL_QUOTAS
            .range(ctx.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for ((user, denom), _) in personal_quotas {
            if frozen_denoms.contains(&denom) {
                PERSONAL_QUOTAS.remove(ctx.storage, (user, &denom));
            }
        }
    }

    rate_limit::apply_admin_update(ctx.storage, ctx.querier, rate_limits)?;

    Ok(Response::new())
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
            let expire_at = available_for
                .map(|d| ctx.block.timestamp.checked_add(d))
                .transpose()?;

            PERSONAL_QUOTAS.save(ctx.storage, (user, &denom), &PersonalQuota {
                amount,
                expire_at,
                granted_by: ctx.sender,
                granted_at: ctx.block.timestamp,
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
        && pq.expire_at.is_none_or(|t| ctx.block.timestamp < t)
    {
        let consumed = pq.amount.min(remaining);
        remaining = remaining.checked_sub(consumed)?;

        let leftover = pq.amount.checked_sub(consumed)?;
        if leftover.is_zero() {
            PERSONAL_QUOTAS.remove(ctx.storage, key);
        } else {
            PERSONAL_QUOTAS.save(ctx.storage, key, &PersonalQuota {
                amount: leftover,
                expire_at: pq.expire_at,
                granted_by: pq.granted_by,
                granted_at: pq.granted_at,
            })?;
        }
    }

    // Check the trailing-24h rolling window against the cap and record the
    // residue. `enforce` short-circuits when the denom is not rate-limited
    // (no `SUPPLY_SNAPSHOTS` entry) or when the residue is zero (the
    // withdraw was fully covered by personal quota).
    rate_limit::enforce(
        ctx.storage,
        &coin.denom,
        ctx.block.timestamp,
        coin.amount,
        remaining,
    )?;

    let (bank, owner) = ctx.querier.query_bank_and_owner()?;

    // 1. Call the bridge contract to make the remote transfer.
    // 2. Burn the alloyed token to be transferred (only if the token is not native on Dango).
    // 3. Send the withdrawal fee to the chain owner.
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
            Some(Message::transfer(owner, coins! { coin.denom => fee })?)
        } else {
            None
        }))
}

pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    rate_limit::tick(ctx.storage, ctx.querier, ctx.block.timestamp)?;

    Ok(Response::new())
}
