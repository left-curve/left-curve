use {
    crate::{CONFIG, VOLUME_TIME_GRANULARITY, VOLUMES_BY_USER, WITHHELD_FEE},
    anyhow::ensure,
    dango_account_factory::AccountQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::{AccountParams, UserIndex},
        bank,
        taxman::{Config, ExecuteMsg, FeeType, InstantiateMsg, ReceiveFee},
    },
    grug::{
        Addr, AuthCtx, AuthMode, Coins, ContractEvent, IsZero, Map, Message, MultiplyFraction,
        MutableCtx, Number, NumberConst, Order, QuerierExt, Response, StdResult, Storage,
        Timestamp, Tx, TxOutcome, Udec128_6, Uint128, coins,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Configure { new_cfg } => configure(ctx, new_cfg),
        ExecuteMsg::Pay { ty, payments } => pay(ctx, ty, payments),
        ExecuteMsg::ReportVolumes(volumes) => report_volumes(ctx, volumes),
    }
}

fn configure(ctx: MutableCtx, new_cfg: Config) -> anyhow::Result<Response> {
    // Only the chain's owner can update fee config.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.save(ctx.storage, &new_cfg)?;

    Ok(Response::new())
}

fn pay(ctx: MutableCtx, ty: FeeType, payments: BTreeMap<Addr, Coins>) -> anyhow::Result<Response> {
    ensure!(ctx.funds.is_non_empty(), "funds cannot be empty!");

    // Ensure funds add up to the total amount of payments.
    let total = payments
        .values()
        .try_fold(Coins::new(), |mut acc, coins| -> StdResult<_> {
            acc.insert_many(coins.clone())?;
            Ok(acc)
        })?;

    for coin in total {
        let paid = ctx.funds.amount_of(&coin.denom);
        ensure!(
            paid >= coin.amount,
            "sent fund is less than declared payment! denom: {}, declared: {}, paid: {}",
            coin.denom,
            coin.amount,
            paid
        );
    }

    // For now, nothing to do.
    // In the future, we will implement affiliate fees.
    let events = payments
        .into_iter()
        .map(|(user, amount)| {
            ContractEvent::new(&ReceiveFee {
                handler: ctx.sender,
                user,
                ty,
                amount,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new().add_events(events)?)
}

fn report_volumes(ctx: MutableCtx, volumes: BTreeMap<Addr, Udec128_6>) -> anyhow::Result<Response> {
    #[cfg(feature = "metrics")]
    let now_volume = std::time::Instant::now();

    let app_cfg = ctx.querier.query_dango_config()?;

    ensure!(
        ctx.sender == app_cfg.addresses.dex,
        "only the dex contract can report volumes"
    );

    // Create account querier.
    let mut account_querier = AccountQuerier::new(app_cfg.addresses.account_factory, ctx.querier);

    // Round the current timestamp _down_ to the nearest day.
    let timestamp = ctx.block.timestamp - ctx.block.timestamp % VOLUME_TIME_GRANULARITY;

    for (user, volume) in volumes {
        // Query the user's account info. If there isn't one (i.e. the user
        // isn't registered through the account factory), skip.
        let Some(account) = account_querier.query_account(user)? else {
            continue;
        };

        // Get the user's user index. If the user is a multisig, skip.
        let AccountParams::Single(params) = &account.params else {
            continue;
        };

        increment_cumulative_volume(
            VOLUMES_BY_USER,
            ctx.storage,
            params.owner,
            timestamp,
            volume,
        )?;
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_DURATION_STORE_VOLUME)
            .record(now_volume.elapsed().as_secs_f64());
    }

    Ok(Response::new())
}

/// Increment the user's cumulative volume.
fn increment_cumulative_volume(
    map: Map<'static, (UserIndex, Timestamp), Udec128_6>,
    storage: &mut dyn Storage,
    user_index: UserIndex,
    timestamp: Timestamp,
    volume: Udec128_6,
) -> StdResult<()> {
    // Find the most recent record of the user's cumulative volume.
    // If not found, default to zero.
    let (existing_timestamp, existing_volume) = map
        .prefix(user_index)
        .range(storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or_default();

    // The existing most recent record shouldn't be newer than the current timestamp.
    // We ensure this in debug mode.
    debug_assert!(
        existing_timestamp <= timestamp,
        "existing cumulative volume has a timestamp newer than the current time: {} > {}",
        existing_timestamp.to_rfc3339_string(),
        timestamp.to_rfc3339_string()
    );

    let new_volume = existing_volume.checked_add(volume)?;

    map.save(storage, (user_index, timestamp), &new_volume)
}

// TODO: exempt the account factory from paying fee.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    let fee_cfg = CONFIG.load(ctx.storage)?;

    // Compute the maximum amount of fee this transaction may incur.
    // Note that we ceil this amount, instead of flooring.
    //
    // Under three situations, we don't charge any gas:
    //
    // 1. During simulation. At this time, the user doesn't know how much gas
    //    gas limit to request. The node's query gas limit is used as `tx.gas_limit`
    //    in this case.
    // 2. Sender is the account factory contract. This happens during a new user
    //    onboarding. We don't charge gas fee this in case.
    // 3. Sender is the oracle contract. Validators supply Pyth price feeds by
    //    using the oracle contract as sender during `PrepareProposal`.
    let withhold_amount = if ctx.mode == AuthMode::Simulate || {
        let app_cfg = ctx.querier.query_dango_config()?;
        tx.sender == app_cfg.addresses.account_factory || tx.sender == app_cfg.addresses.oracle
    } {
        Uint128::ZERO
    } else {
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?
    };

    // If the withhold amount is non-zero, we force transfer this amount from
    // the sender to taxman.
    //
    // If the sender doesn't have enough fund to cover the maximum amount of fee
    // the tx may incur, this submessage fails, causing the tx to be rejected
    // from entering the mempool.
    let withhold_msg = if withhold_amount.is_non_zero() {
        let bank = ctx.querier.query_bank()?;
        Some(Message::execute(
            bank,
            &bank::ExecuteMsg::ForceTransfer {
                from: tx.sender,
                to: ctx.contract,
                coins: coins! { fee_cfg.fee_denom.clone() => withhold_amount },
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    // Save the withheld fee in storage, which we will use in `finalize_fee`.
    WITHHELD_FEE.save(ctx.storage, &(fee_cfg, withhold_amount))?;

    Ok(Response::new().may_add_message(withhold_msg))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn finalize_fee(ctx: AuthCtx, tx: Tx, outcome: TxOutcome) -> StdResult<Response> {
    let (fee_cfg, withheld_amount) = WITHHELD_FEE.take(ctx.storage)?;

    // Compute how much fee to charge the sender, based on the actual amount of
    // gas consumed.
    //
    // Again, during simulation, or any tx sent by the account factory, is
    // exempt from gas fees.
    let charge_amount = if ctx.mode == AuthMode::Simulate || {
        let app_cfg = ctx.querier.query_dango_config()?;
        tx.sender == app_cfg.addresses.account_factory || tx.sender == app_cfg.addresses.oracle
    } {
        Uint128::ZERO
    } else {
        Uint128::new(outcome.gas_used as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?
    };

    // If we have withheld more funds than the actual charge amount, we need to
    // refund the difference.
    let refund_amount = withheld_amount.saturating_sub(charge_amount);

    // Use ForceTransfer instead of Transfer so that we don't need to invoke the
    // sender's `receive` method (unnecessary).
    let refund_msg = if refund_amount.is_non_zero() {
        let bank = ctx.querier.query_bank()?;
        Some(Message::execute(
            bank,
            &bank::ExecuteMsg::ForceTransfer {
                from: ctx.contract,
                to: tx.sender,
                coins: coins! { fee_cfg.fee_denom.clone() => refund_amount },
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    Ok(Response::new().may_add_message(refund_msg))
}
