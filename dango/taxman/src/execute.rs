use {
    crate::{
        CONFIG, FEE_SHARE_RATIO, MAX_REFERRER_CHAIN_DEPTH, REFEREE_TO_REFERRER,
        REFERRER_TO_REFEREE_STATISTICS, USER_REFERRAL_DATA, VOLUMES_BY_USER, WITHHELD_FEE,
    },
    anyhow::{bail, ensure},
    dango_account_factory::AccountQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::{
            AccountParams, QueryAccountRequest, QueryAccountsByUserRequest, UserIndex,
            UserIndexOrName,
        },
        bank,
        taxman::{
            CommissionRebund, Config, ExecuteMsg, FeeType, InstantiateMsg, ReceiveFee, Referee,
            RefereeData, Referral, Referrer, ReferrerInfo, ShareRatio, UserReferralData,
        },
    },
    grug::{
        Addr, AuthCtx, AuthMode, Coin, Coins, ContractEvent, Duration, Inner, IsZero, Map, Message,
        MultiplyFraction, MutableCtx, Number, NumberConst, Order, QuerierExt, Response, StdError,
        StdResult, Storage, Timestamp, Tx, TxOutcome, Udec128, Udec128_6, Uint128, coins,
    },
    std::{collections::BTreeMap, ops::Mul},
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
        ExecuteMsg::SetReferral { referrer, referee } => set_referral(ctx, referrer, referee),
        ExecuteMsg::SetFeeShareRatio(bounded) => set_share_ratio(ctx, bounded),
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
    let events = payments
        .clone()
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

    // If the FeeType is Trade, create Messages for rebound fee.
    let msgs = if ty == FeeType::Trade {
        fee_rebound(ctx, payments)?
    } else {
        Vec::new()
    };

    Ok(Response::new().add_messages(msgs).add_events(events)?)
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
    let day_timestamp = ctx.block.timestamp.truncate_to_days();

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
            day_timestamp,
            volume,
        )?;

        // Store the volume for the referral program.
        let Some(referrer_info) = referrer_info(&ctx, params.owner)? else {
            continue;
        };

        // Update the total volume the referee has traded for the referrer.
        REFERRER_TO_REFEREE_STATISTICS.update(
            ctx.storage,
            (referrer_info.user, params.owner),
            |mut data| {
                data.volume.checked_add_assign(volume)?;
                Ok::<_, StdError>(data)
            },
        )?;

        // Update the cumulative volume for the referee.
        // NOTE: This is not the total volume the user has traded, but only the volume
        // the user traded since he has a referrer.
        let mut referee_data = last_user_data(ctx.storage, params.owner)?;
        referee_data.volume.checked_add_assign(volume)?;
        USER_REFERRAL_DATA.save(ctx.storage, (params.owner, day_timestamp), &referee_data)?;
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_DURATION_STORE_VOLUME)
            .record(now_volume.elapsed().as_secs_f64());
    }

    Ok(Response::new())
}

fn set_referral(ctx: MutableCtx, referrer: Referrer, referee: Referee) -> anyhow::Result<Response> {
    // Ensure referrer and referee are not the same.
    ensure!(
        referrer != referee,
        "referrer and referee cannot be the same"
    );

    // Ensure the referrer has set the fee share ratio.
    ensure!(
        FEE_SHARE_RATIO.may_load(ctx.storage, referrer)?.is_some(),
        "user {referrer} has not set fee share ratio"
    );

    // Ensure the caller is either the account factory or the referee himself.
    let account_factory = ctx.querier.query_account_factory()?;

    if ctx.sender != account_factory {
        // Retrieve the user index of the sender.
        let sender_user_index =
            match ctx
                .querier
                .query_wasm_smart(account_factory, QueryAccountRequest {
                    address: ctx.sender,
                }) {
                Ok(account) => {
                    let AccountParams::Single(account_params) = account.params else {
                        bail!("only single accounts can set referral");
                    };

                    account_params.owner
                },
                Err(_) => {
                    bail!("unable to retrieve account info for address {}", ctx.sender);
                },
            };

        ensure!(
            sender_user_index == referee,
            "only account factory or the referee himself can set referral"
        );
    }

    REFEREE_TO_REFERRER.may_update(ctx.storage, referee, |maybe_referrer| {
        // If the referral is already set, it's not allowed to change.
        if maybe_referrer.is_some() {
            anyhow::bail!("user {referee} already has a referrer and it can't be changed",);
        }
        Ok(referrer)
    })?;

    REFERRER_TO_REFEREE_STATISTICS.save(ctx.storage, (referrer, referee), &RefereeData {
        registered_at: ctx.block.timestamp,
        volume: Udec128::ZERO,
        commission_rebounded: Udec128::ZERO,
    })?;

    let mut referrer_data = last_user_data(ctx.storage, referrer)?;
    referrer_data.referee_count += 1;

    // Get the timestamp rounded down to the nearest day.
    let day_timestamp = ctx.block.timestamp.truncate_to_days();

    USER_REFERRAL_DATA.save(ctx.storage, (referrer, day_timestamp), &referrer_data)?;

    Ok(Response::new().add_event(Referral { referrer, referee })?)
}

fn set_share_ratio(ctx: MutableCtx, rate: ShareRatio) -> anyhow::Result<Response> {
    let account_params = ctx
        .querier
        .query_wasm_smart(ctx.querier.query_account_factory()?, QueryAccountRequest {
            address: ctx.sender,
        })?
        .params;

    let AccountParams::Single(params) = account_params else {
        bail!("only single accounts can set fee share ratio");
    };

    // In order to set the share ratio and be a referrer, the user must have
    // traded at least 10k.
    let traded_volume = VOLUMES_BY_USER
        .prefix(params.owner)
        .values(ctx.storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_6::ZERO);

    let volume_to_be_referrer = CONFIG.load(ctx.storage)?.referral.volume_to_be_referrer;

    ensure!(
        traded_volume.into_int() >= volume_to_be_referrer,
        "you must have at least a volume of ${} to become a referrer, traded volume: ${}",
        volume_to_be_referrer,
        traded_volume.into_int()
    );

    FEE_SHARE_RATIO.may_update(ctx.storage, params.owner, |maybe_rate| {
        if let Some(existing_rate) = maybe_rate {
            ensure!(
                rate.inner() >= existing_rate.inner(),
                "can only increase fee share ratio, existing: {}, new: {}",
                existing_rate.inner(),
                rate.inner()
            );
        }
        Ok(rate)
    })?;

    Ok(Response::new())
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

// --------------------------------------- Volume functions ---------------------------------------

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

// ---------------------------------- Referral Program functions ----------------------------------

// Process fee rebound for trade fees.
fn fee_rebound(ctx: MutableCtx, payments: BTreeMap<Addr, Coins>) -> anyhow::Result<Vec<Message>> {
    let mut msgs = Vec::new();

    // Calculate the timestamp rounded down to the current day.
    let day_timestamp = ctx.block.timestamp.truncate_to_days();

    let account_factory = ctx.querier.query_account_factory()?;
    let mut account_querier = AccountQuerier::new(account_factory, ctx.querier);
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);

    for (address, coins) in payments {
        let Some(account) = account_querier.query_account(address)? else {
            continue;
        };

        // The rebate only applies to single accounts.
        let AccountParams::Single(payer_account_params) = account.params.clone() else {
            continue;
        };

        // Create the referrer chain, in order to calculate the rebounded fee.
        // The payer fee reboud is calculated as first_referrer_commission_rebund * first_referrer_share_ratio.
        // All the subsequent referrers the rebound fee percentage is calculated as referrer_fee_commission_rebund - max_referrer_commission_rebund.
        // E.g.:
        // - first referrer:  10% commission rebund, 50% share ratio
        // - second referrer: 10% commission rebund
        // - third referrer:  25% commission rebund
        //
        // The payer gets 5% rebounded fee (10% * 50%).
        // The first referrer gets 5% rebounded fee (10% - 5%).
        // The second referrer gets 0% rebounded fee (10% - 10%).
        // The third referrer gets 15% rebounded fee (25% - 10%).

        // Retrieve the first referrer info. We keep the first referrer outside from the other referrers
        // since we need to store extra data for him.
        // If the payer doesn't have a referrer, skip rebounding.
        let Some(first_referrer) = referrer_info(&ctx, payer_account_params.owner)? else {
            continue;
        };

        let mut last_referee = first_referrer.user;
        let mut referrer_chain = Vec::with_capacity(MAX_REFERRER_CHAIN_DEPTH as usize + 1);
        // Retrieve MAX_REFERRER_CHAIN_DEPTH - 1 since we have already retrieved the first referrer.
        for _ in 0..MAX_REFERRER_CHAIN_DEPTH - 1 {
            let Some(referrer_info) = referrer_info(&ctx, last_referee)? else {
                break;
            };

            last_referee = referrer_info.user;
            referrer_chain.push(referrer_info);
        }

        // Calculate the commission rebound for the payer.
        let payer_commission_rebund = first_referrer
            .commission_rebund
            .into_inner()
            .mul(first_referrer.share_ratio.into_inner());

        // Calculate the rebounded coins for the payer.
        let commission_rebound_value = calculate_and_send_commission_rebound(
            &coins,
            CommissionRebund::new(payer_commission_rebund)?,
            &mut oracle_querier,
            get_main_account(&ctx, account_factory, payer_account_params.owner)?,
            &mut msgs,
        )?;

        if commission_rebound_value.is_non_zero() {
            // Retrieve the most recent record of the user's cumulative data.
            let mut payer_data = last_user_data(ctx.storage, payer_account_params.owner)?;

            // Increase the commission rebounded value.
            payer_data
                .commission_rebounded
                .checked_add_assign(commission_rebound_value)?;

            USER_REFERRAL_DATA.save(
                ctx.storage,
                (payer_account_params.owner, day_timestamp),
                &payer_data,
            )?;
        }

        // Calculate the commission rebound for the first referrer.
        let commission_rebound = *first_referrer.commission_rebund - payer_commission_rebund;

        // Calculate the rebounded coins for the first referrer.
        let commission_rebound_value = calculate_and_send_commission_rebound(
            &coins,
            CommissionRebund::new(commission_rebound)?,
            &mut oracle_querier,
            get_main_account(&ctx, account_factory, first_referrer.user)?,
            &mut msgs,
        )?;

        if commission_rebound_value.is_non_zero() {
            // Store the referee commission rebounded value.
            store_referee_commission_rebound(
                ctx.storage,
                first_referrer.user,
                day_timestamp,
                commission_rebound_value,
            )?;

            // Update the total referee commission rebound for the referrer.
            REFERRER_TO_REFEREE_STATISTICS.update(
                ctx.storage,
                (first_referrer.user, payer_account_params.owner),
                |mut data| {
                    data.commission_rebounded
                        .checked_add_assign(commission_rebound_value)?;
                    Ok::<_, StdError>(data)
                },
            )?;
        }

        // Max commission rate seen so far in the referrer chain.
        let mut max_commission_rate = *first_referrer.commission_rebund;

        // Iterate through the referrer chain to distribute the fee.
        for referrer_info in referrer_chain {
            // Check if this referrer is eligible for rebounding.
            if *referrer_info.commission_rebund <= max_commission_rate {
                continue;
            }

            // Calculate the effective commission rate for this referrer.
            let commission_rate = referrer_info
                .commission_rebund
                .into_inner()
                .saturating_sub(max_commission_rate);

            max_commission_rate = *referrer_info.commission_rebund;

            // Calculate the rebounded coins for this referrer.
            let commission_rebound_value = calculate_and_send_commission_rebound(
                &coins,
                CommissionRebund::new(commission_rate)?,
                &mut oracle_querier,
                get_main_account(&ctx, account_factory, referrer_info.user)?,
                &mut msgs,
            )?;

            // Store the referee commission rebounded value.
            if commission_rebound_value.is_non_zero() {
                store_referee_commission_rebound(
                    ctx.storage,
                    referrer_info.user,
                    day_timestamp,
                    commission_rebound_value,
                )?;
            }
        }
    }
    Ok(msgs)
}

// Given a user index, retrieve his main account address.
fn get_main_account(
    ctx: &MutableCtx,
    account_factory: Addr,
    user: UserIndex,
) -> anyhow::Result<Addr> {
    // TODO: Once the Main address branch is merged, query the main address.
    let accounts = ctx
        .querier
        .query_wasm_smart(account_factory, QueryAccountsByUserRequest {
            user: UserIndexOrName::Index(user),
        })?;
    let (user_address, _) = accounts.first_key_value().unwrap();

    Ok(*user_address)
}

// Retrieve the most recent record of the user's cumulative data.
/// If none exists, return the default value.
fn last_user_data(storage: &dyn Storage, user: UserIndex) -> anyhow::Result<UserReferralData> {
    let (_, data) = USER_REFERRAL_DATA
        .prefix(user)
        .range(storage, None, None, Order::Descending)
        .next()
        .transpose()?
        .unwrap_or_default();

    Ok(data)
}

// Update the referee commission rebound value for a user.
fn store_referee_commission_rebound(
    storage: &mut dyn Storage,
    user: UserIndex,
    day_timestamp: Duration,
    commission_rebound_value: Udec128,
) -> anyhow::Result<()> {
    // Retrieve the most recent record of the user's cumulative data.
    let mut user_data = last_user_data(storage, user)?;

    // Increase the commission rebounded value.
    user_data
        .referees_commission_rebounded
        .checked_add_assign(commission_rebound_value)?;

    USER_REFERRAL_DATA.save(storage, (user, day_timestamp), &user_data)?;

    Ok(())
}

// Calculate the rebounded coins and create the Transfer Msg.
/// Return the rebounded value in USD.
fn calculate_and_send_commission_rebound(
    coins: &Coins,
    commission_rebound: CommissionRebund,
    oracle_querier: &mut OracleQuerier,
    receiver: Addr,
    msgs: &mut Vec<Message>,
) -> anyhow::Result<Udec128> {
    let mut rebound_coins = Coins::new();
    let mut commission_value = Udec128::ZERO;

    for coin in coins {
        let rebounded_amount = coin.amount.checked_mul_dec_floor(*commission_rebound)?;

        if rebounded_amount.is_zero() {
            continue;
        }

        rebound_coins.insert(Coin::new(coin.denom.clone(), rebounded_amount)?)?;

        let price = oracle_querier.query_price(coin.denom, None)?;
        let value: Udec128 = price.value_of_unit_amount(rebounded_amount)?;
        commission_value.checked_add_assign(value)?;
    }

    // Create transfer message if there are coins to rebound.
    if !rebound_coins.is_empty() {
        msgs.push(Message::transfer(receiver, rebound_coins)?);
    }

    Ok(commission_value)
}

// Given a referee, return his referrer and the fee share ratio, if any.
fn referrer_info(ctx: &MutableCtx, referee: Referee) -> anyhow::Result<Option<ReferrerInfo>> {
    if let Some(referrer) = REFEREE_TO_REFERRER.may_load(ctx.storage, referee)?
        && let Some(share_ratio) = FEE_SHARE_RATIO.may_load(ctx.storage, referrer)?
    {
        let commission_rebund = calculate_commission_rebund(ctx, referrer)?;
        return Ok(Some(ReferrerInfo {
            user: referrer,
            commission_rebund,
            share_ratio,
        }));
    }

    Ok(None)
}

/// Calculate the commission rebound ratio for a referrer.
fn calculate_commission_rebund(
    ctx: &MutableCtx,
    referrer: Referrer,
) -> anyhow::Result<CommissionRebund> {
    // Retrieve the last user data for the referrer.
    let data_last = last_user_data(ctx.storage, referrer)?;

    let since = ctx
        .block
        .timestamp
        .saturating_sub(Duration::from_days(30))
        .truncate_to_days();

    // Retrieve the user data since 30 days ago.
    let data_since = USER_REFERRAL_DATA
        .prefix(referrer)
        .values(
            ctx.storage,
            None,
            Some(grug::Bound::Inclusive(since)),
            Order::Descending,
        )
        .next()
        .transpose()?
        .unwrap_or_default();

    // Calculate the volume the referees traded in the last 30 days.
    let referees_volume = data_last
        .referees_volume
        .checked_sub(data_since.referees_volume)?;

    // Determine the commission rebund ratio based on the referees volume.
    let referral_config = CONFIG.load(ctx.storage)?.referral;

    let mut referrer_commission_rebound = referral_config.commission_rebound_default;

    for (volume_threshold, commission_rebound) in referral_config.commission_rebound_by_volume {
        if referees_volume.into_int() >= volume_threshold {
            referrer_commission_rebound = commission_rebound;
        } else {
            break;
        }
    }

    Ok(referrer_commission_rebound)
}
