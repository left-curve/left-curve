use {
    crate::{
        ASKS, BIDS, PAIR_IDS, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{compute_funding_delta, compute_impact_price, compute_premium},
        execute::ORACLE,
        price::may_invert_price,
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        Days, UsdPrice, UsdValue,
        perps::{PairId, UserState, settlement_currency},
    },
    grug::{
        Addr, Coins, Message, Order as IterationOrder, PrefixBound, Response, StdResult, Storage,
        SudoCtx, Timestamp, TransferBuilder,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    let maybe_payout = process_unlocks(ctx.storage, ctx.block.timestamp, &mut oracle_querier)?;

    process_funding(ctx.storage, ctx.block.timestamp, &mut oracle_querier)?;

    Ok(Response::new().may_add_message(maybe_payout))
}

/// Pop matured unlocks from each user and compute the amount of settlement
/// currency to release.
fn process_unlocks(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<Option<Message>> {
    let mut transfers = TransferBuilder::<Coins>::new();

    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // Load all users whose earliest unlock has matured.
    let users = USER_STATES
        .idx
        .earliest_unlock_end_time
        .prefix_range(
            storage,
            None,
            Some(PrefixBound::Inclusive(current_time)),
            IterationOrder::Ascending,
        )
        .map(|res| {
            let (_timestamp, user, user_state) = res?;
            Ok((user, user_state))
        })
        .collect::<StdResult<Vec<_>>>()?;

    for (user, user_state) in users {
        process_unlock_for_user(
            storage,
            current_time,
            settlement_currency_price,
            user,
            user_state,
            &mut transfers,
        )?;
    }

    if transfers.is_empty() {
        Ok(None)
    } else {
        Ok(Some(transfers.into_message()))
    }
}

fn process_unlock_for_user(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    settlement_currency_price: UsdPrice,
    user: Addr,
    mut user_state: UserState,
    transfers: &mut TransferBuilder<Coins>,
) -> anyhow::Result<()> {
    let mut amount_usd = UsdValue::ZERO;

    // Loop through unlocks, pop the ones that have matured, sum up USD value
    // of all that have matured.
    while let Some(unlock) = user_state.unlocks.front() {
        if unlock.end_time > current_time {
            break;
        }

        amount_usd.checked_add_assign(unlock.amount_to_release)?;
        user_state.unlocks.pop_front();
    }

    // Convert the total USD amount to amount of the settlement currency token.
    let amount_token = amount_usd
        .checked_div(settlement_currency_price)?
        .into_base_floor(settlement_currency::DECIMAL)?;

    // Insert the tokens into pending transfer queue.
    // No need to check whether `amount_token` != 0 here. The `insert` function handles this.
    transfers.insert(user, settlement_currency::DENOM.clone(), amount_token)?;

    // Save the updated user state to storage.
    if user_state.is_empty() {
        USER_STATES.remove(storage, user)?;
    } else {
        USER_STATES.save(storage, user, &user_state)?;
    }

    Ok(())
}

/// Compute and apply funding deltas for each trading pair using a point-in-time
/// premium snapshot from the order book.
fn process_funding(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    let param = PARAM.load(storage)?;
    let mut state = STATE.load(storage)?;

    // Only process funding if sufficient time has elapsed since the last funding time.
    let elapsed = current_time - state.last_funding_time;
    if elapsed < param.funding_period {
        return Ok(());
    }

    let interval = Days::from_duration(elapsed)?;
    let pair_ids = PAIR_IDS.load(storage)?;

    for pair_id in pair_ids {
        process_funding_for_pair(storage, oracle_querier, interval, pair_id)?;
    }

    state.last_funding_time = current_time;

    STATE.save(storage, &state)?;

    Ok(())
}

fn process_funding_for_pair(
    storage: &mut dyn Storage,
    oracle_querier: &mut OracleQuerier,
    interval: Days,
    pair_id: PairId,
) -> anyhow::Result<()> {
    let pair_param = PAIR_PARAMS.load(storage, &pair_id)?;
    let mut pair_state = PAIR_STATES.load(storage, &pair_id)?;

    let oracle_price = oracle_querier.query_price_for_perps(&pair_id)?;

    // Walk the bid side: stored in ascending order of inverted price,
    // so ascending iteration gives best-bid-first. Un-invert to get
    // real prices. Bid order sizes are guaranteed positive.
    let bid_iter = BIDS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((stored_price, _), order) = res?;
            let real_price = may_invert_price(stored_price, true);
            Ok((real_price, order.size))
        });

    // Walk the ask side: stored naturally in ascending price order,
    // so ascending iteration gives best-ask-first.
    let ask_iter = ASKS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((stored_price, _), order) = res?;
            Ok((stored_price, order.size.checked_abs()?))
        });

    let impact_bid = compute_impact_price(bid_iter, pair_param.impact_notional)?;
    let impact_ask = compute_impact_price(ask_iter, pair_param.impact_notional)?;

    let premium = compute_premium(impact_bid, impact_ask, oracle_price)?;

    let funding_delta = compute_funding_delta(
        premium,
        oracle_price,
        pair_param.max_abs_funding_rate,
        interval,
    )?;

    (pair_state.funding_per_unit).checked_add_assign(funding_delta)?;

    PAIR_STATES.save(storage, &pair_id, &pair_state)?;

    Ok(())
}
