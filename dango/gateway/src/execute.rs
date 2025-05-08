use {
    crate::{OUTBOUND_QUOTAS, RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, WITHDRAWAL_FEES},
    anyhow::{anyhow, ensure},
    dango_types::{
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, RateLimit, Remote, WithdrawalFee,
            bridge::{self, BridgeMsg},
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, Denom, Inner, Message, MultiplyFraction, MutableCtx, Number, NumberConst,
        Part, QuerierExt, Response, StdError, StdResult, Storage, SudoCtx, Uint128, btree_map,
        coins,
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

fn set_routes(ctx: MutableCtx, routes: BTreeSet<(Part, Addr, Remote)>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set routes"
    );

    _set_routes(ctx.storage, routes)?;

    Ok(Response::new())
}

fn _set_routes(storage: &mut dyn Storage, routes: BTreeSet<(Part, Addr, Remote)>) -> StdResult<()> {
    for (part, bridge, remote) in routes {
        let denom = Denom::from_parts([NAMESPACE.clone(), part])?;

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
        WITHDRAWAL_FEES.save(storage, (&denom, remote), &fee)?;
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

    // Increase the reserve.
    RESERVES.may_update(ctx.storage, (ctx.sender, remote), |maybe_reserve| {
        let reserve = maybe_reserve.unwrap_or(Uint128::ZERO);
        Ok::<_, StdError>(reserve.checked_add(amount)?)
    })?;

    // Increase the outbound quota.
    OUTBOUND_QUOTAS.may_update(ctx.storage, &denom, |maybe_quota| {
        let quota = maybe_quota.unwrap_or(Uint128::MAX);
        Ok::<_, StdError>(quota.checked_add(amount)?)
    })?;

    // Mint the alloyed token to the recipient.
    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: recipient,
                coins: coins! { denom => amount },
            },
            Coins::new(),
        )?
    }))
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

    // Reduce the reserve.
    RESERVES.update(ctx.storage, (bridge, remote), |reserve| {
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

    // Reduce the outbound quota.
    OUTBOUND_QUOTAS.update(ctx.storage, &coin.denom, |quote| {
        quote.checked_sub(coin.amount).map_err(|_| {
            anyhow!(
                "insufficient outbound quota! denom: {}, amount: {}",
                coin.denom,
                coin.amount
            )
        })
    })?;

    // 1. Call the bridge contract to make the remote transfer.
    // 2. Pay fee to the taxman.
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
        .may_add_message(if let Some(fee) = maybe_fee {
            let taxman = ctx.querier.query_taxman()?;
            Some(Message::execute(
                taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Withdraw,
                    payments: btree_map! {
                        ctx.sender => coins! { coin.denom => fee },
                    },
                },
                Coins::new(),
            )?)
        } else {
            None
        }))
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
