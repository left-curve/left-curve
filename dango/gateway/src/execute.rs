use {
    crate::{
        EPOCH, GLOBAL_OUTBOUND, RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, SUPPLIES,
        USER_MOVEMENTS, WITHDRAWAL_CREDITS, WITHDRAWAL_FEES,
    },
    anyhow::{anyhow, ensure},
    dango_types::{
        DangoQuerier,
        account_factory::{self, UserIndex},
        bank,
        gateway::{
            Addr32, ExecuteMsg, InstantiateMsg, NAMESPACE, Origin, RateLimit, Remote, Traceable,
            WINDOW_SIZE, WithdrawalCredit, WithdrawalFee,
            bridge::{self, BridgeMsg},
        },
        taxman::{self, FeeType},
    },
    grug::{
        Addr, Coins, Denom, Inner, Message, MutableCtx, Number, NumberConst, Op, QuerierExt,
        QuerierWrapper, Response, StdError, StdResult, Storage, SudoCtx, Udec128, Uint128,
        btree_map, coins,
    },
    std::collections::{BTreeMap, BTreeSet},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    _set_routes(ctx.storage, msg.routes)?;
    _set_rate_limits(ctx.storage, msg.rate_limits)?;
    _set_withdrawal_fees(ctx.storage, msg.withdrawal_fees)?;

    EPOCH.save(ctx.storage, &0)?;

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
        ExecuteMsg::SetWithdrawalCredit {
            user_index,
            denom,
            credit,
        } => set_withdrawal_credit(ctx, user_index, denom, credit),
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
    // waiting for the next cron cycle. Existing snapshots and outbound windows
    // are left untouched. The updated withdrawal rate takes effect immediately
    // — both increases and decreases — because the check compares the new
    // allowance against the current rolling outbound.
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

fn set_withdrawal_credit(
    ctx: MutableCtx,
    user_index: UserIndex,
    denom: Denom,
    credit: Op<(Uint128, grug::Duration)>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can set withdrawal credits"
    );

    match credit {
        Op::Insert((amount, duration)) => {
            let expires_at = ctx.block.timestamp.checked_add(duration)?;

            WITHDRAWAL_CREDITS.save(ctx.storage, (user_index, &denom), &WithdrawalCredit {
                amount,
                used: Uint128::ZERO,
                expires_at,
            })?;
        },
        Op::Delete => {
            WITHDRAWAL_CREDITS.remove(ctx.storage, (user_index, &denom));
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

    // Track the deposit in the recipient's per-user movement for historical
    // observability (and potential future trust-tier logic).
    //
    // This runs for all denoms, not just rate-limited ones. The extra storage
    // cost is intentional: we want historical deposit data available even for
    // denoms that may become rate-limited in the future.
    //
    // Deposits must always succeed regardless of the recipient's registration
    // status. If the recipient is not registered in the account factory (e.g.
    // an unknown address), the funds are still minted and
    // transferred — they will be held as an orphan transfer in the bank until
    // the address is claimed or the funds retrieved by the owner.
    // In that case we simply skip movement tracking.
    {
        if let Ok(user_index) = resolve_user_index(ctx.querier, recipient) {
            let mut movement = USER_MOVEMENTS
                .may_load(ctx.storage, (user_index, &denom))?
                .unwrap_or_default();
            movement.deposited.checked_add_assign(amount)?;
            USER_MOVEMENTS.save(ctx.storage, (user_index, &denom), &movement)?;
        };
    }

    #[cfg(feature = "metrics")]
    {
        metrics::gauge!(crate::metrics::LABEL_DEPOSITS, "denom" => denom.to_string())
            .increment(amount.into_inner() as f64);
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
    // the entire withdrawal counts against the global outbound, which must
    // not exceed `supply * rate_limit`.
    //
    // Only account-factory-registered users can withdraw from rate-limited
    // denoms. This is intentional: all legitimate users are registered, and
    // contracts are not expected to call `transfer_remote` directly. If
    // contract-initiated withdrawals are needed in the future, a whitelist
    // mechanism should be added.
    if let Some(rate_limit) = RATE_LIMITS.load(ctx.storage)?.get(&coin.denom) {
        // A zero rate limit acts as an emergency freeze — all withdrawals are
        // blocked. This lets the owner halt outflows immediately in response
        // to a bridge exploit.
        ensure!(
            rate_limit.into_inner() > Udec128::ZERO,
            "withdrawals are frozen for denom: {}",
            coin.denom
        );

        let user_index = resolve_user_index(ctx.querier, ctx.sender)?;

        // Check if the user has an active withdrawal credit. If so, use it
        // first — the covered portion bypasses the global rate limit.
        let mut withdraw_amount = coin.amount;

        if let Some(mut credit) =
            WITHDRAWAL_CREDITS.may_load(ctx.storage, (user_index, &coin.denom))?
        {
            let credit_available = credit.remaining(ctx.block.timestamp)?;
            let credit_consumed = credit_available.min(withdraw_amount);

            // Update the credit consumed and decrease the withdraw_amount.
            if credit_consumed > Uint128::ZERO {
                credit.used.checked_add_assign(credit_consumed)?;
                withdraw_amount = withdraw_amount.checked_sub(credit_consumed)?;
            }

            // Delete the credit if it's expired or fully consumed.
            if credit.remaining(ctx.block.timestamp)? == Uint128::ZERO {
                WITHDRAWAL_CREDITS.remove(ctx.storage, (user_index, &coin.denom));
            } else if credit_consumed > Uint128::ZERO {
                WITHDRAWAL_CREDITS.save(ctx.storage, (user_index, &coin.denom), &credit)?;
            }
        }

        // Any amount not covered by credit counts against the global limit.
        if withdraw_amount > Uint128::ZERO {
            let mut global = GLOBAL_OUTBOUND
                .may_load(ctx.storage, &coin.denom)?
                .unwrap_or_default();

            let global_available = crate::query::compute_global_available_withdraw(
                ctx.storage,
                &coin.denom,
                rate_limit,
                global.total_24h,
            )?;

            ensure!(
                global_available >= withdraw_amount,
                "rate limit exceeded! denom: {}, available: {}, amount: {}",
                coin.denom,
                global_available,
                withdraw_amount
            );

            global.add_to_current(withdraw_amount)?;
            GLOBAL_OUTBOUND.save(ctx.storage, &coin.denom, &global)?;
        }

        // Record the withdrawal in per-user movement for observability.
        let mut movement = USER_MOVEMENTS
            .may_load(ctx.storage, (user_index, &coin.denom))?
            .unwrap_or_default();

        movement.withdrawn.checked_add_assign(coin.amount)?;

        USER_MOVEMENTS.save(ctx.storage, (user_index, &coin.denom), &movement)?;
    }

    #[cfg(feature = "metrics")]
    {
        metrics::gauge!(crate::metrics::LABEL_WITHDRAWALS, "denom" => coin.denom.to_string())
            .increment(coin.amount.into_inner() as f64);
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
    // Advance to the next epoch (hourly).
    let epoch = EPOCH.update(ctx.storage, |epoch| Ok::<_, StdError>(epoch + 1))?;

    let rate_limits = RATE_LIMITS.load(ctx.storage)?;

    // Rotate the sliding window for each rate-limited denom: pop the oldest
    // hourly slot and push a fresh zero slot. O(1) per denom.
    for denom in rate_limits.keys() {
        let mut global = GLOBAL_OUTBOUND
            .may_load(ctx.storage, denom)?
            .unwrap_or_default();

        global.rotate()?;

        GLOBAL_OUTBOUND.save(ctx.storage, denom, &global)?;
    }

    // Every WINDOW_SIZE epochs (once per day), re-snapshot the supply so the
    // daily allowance reflects current on-chain balances.
    if epoch.is_multiple_of(WINDOW_SIZE) {
        SUPPLIES.clear(ctx.storage, None, None);

        for denom in rate_limits.keys() {
            let supply = ctx.querier.query_supply(denom.clone())?;
            SUPPLIES.save(ctx.storage, denom, &supply)?;
        }

        // Clean up expired withdrawal credits.
        let expired: Vec<_> = WITHDRAWAL_CREDITS
            .range(ctx.storage, None, None, grug::Order::Ascending)
            .filter_map(|res| {
                let (key, credit) = res.ok()?;
                (ctx.block.timestamp >= credit.expires_at).then_some(key)
            })
            .collect();

        for (user_index, denom) in expired {
            WITHDRAWAL_CREDITS.remove(ctx.storage, (user_index, &denom));
        }
    }

    Ok(Response::new())
}

/// Resolves an address to its owning user index via the account factory.
// TODO: optimization: define factory as a const; use raw query instead of smart.
fn resolve_user_index(querier: QuerierWrapper, address: Addr) -> StdResult<UserIndex> {
    let factory = querier.query_account_factory()?;

    querier
        .query_wasm_smart(factory, account_factory::QueryAccountRequest { address })
        .map(|account| account.owner)
}
