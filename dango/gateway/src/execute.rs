use {
    crate::{OUTBOUND, RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, SUPPLIES, WITHDRAWAL_FEES},
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, Origin, RateLimit, Remote, Traceable,
            WithdrawalFee,
            bridge::{self, BridgeMsg},
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, Denom, Inner, Message, MultiplyFraction, MutableCtx, Number, NumberConst, Op,
        QuerierExt, Response, StdError, StdResult, Storage, SudoCtx, Uint128, btree_map, coins,
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

    // Snapshot the current supply for any denom that doesn't already have a
    // snapshot, so newly added rate limits are enforced immediately without
    // waiting for the next cron cycle. Existing snapshots (and outbound
    // accumulators) are left untouched so that lowering a rate limit takes
    // effect instantly.
    for denom in rate_limits.keys() {
        if !SUPPLIES.has(ctx.storage, denom) {
            let supply = ctx.querier.query_supply(denom.clone())?;
            SUPPLIES.save(ctx.storage, denom, &supply)?;
        }
    }

    _set_rate_limits(ctx.storage, rate_limits)?;

    Ok(Response::new())
}

fn _set_rate_limits(
    storage: &mut dyn Storage,
    rate_limits: BTreeMap<Denom, RateLimit>,
) -> StdResult<()> {
    RATE_LIMITS.save(storage, &rate_limits)?;

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

    // Check the rate limit. If a rate limit is configured for this denom,
    // verify that the total outbound for the current window (including this
    // transfer) does not exceed `snapshotted_supply * rate_limit`.
    if let Some(rate_limit) = RATE_LIMITS.load(ctx.storage)?.get(&coin.denom) {
        let supply = SUPPLIES.load(ctx.storage, &coin.denom)?;
        let daily_allowance = supply.checked_mul_dec_floor(rate_limit.into_inner())?;

        let outbound = OUTBOUND
            .may_load(ctx.storage, &coin.denom)?
            .unwrap_or(Uint128::ZERO);
        let new_outbound = outbound.checked_add(coin.amount)?;

        ensure!(
            daily_allowance >= new_outbound,
            "rate limit exceeded! denom: {}, daily_allowance: {}, outbound: {}",
            coin.denom,
            daily_allowance,
            new_outbound
        );

        OUTBOUND.save(ctx.storage, &coin.denom, &new_outbound)?;
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
    // Reset the outbound accumulators for the new 24-hour window.
    OUTBOUND.clear(ctx.storage, None, None);

    // Snapshot the current supply for each rate-limited denom so the daily
    // daily allowance (`supply * rate_limit`) is fixed for the entire window.
    SUPPLIES.clear(ctx.storage, None, None);

    for (denom, _) in RATE_LIMITS.load(ctx.storage)? {
        let supply = ctx.querier.query_supply(denom.clone())?;
        SUPPLIES.save(ctx.storage, &denom, &supply)?;
    }

    Ok(Response::new())
}
