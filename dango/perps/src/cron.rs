use {
    crate::{
        ASKS, BIDS, PAIR_IDS, PAIR_PARAMS, PAIR_STATES,
        core::{compute_funding_delta, compute_impact_price, compute_premium},
        execute::ORACLE,
        price::may_invert_price,
    },
    dango_oracle::OracleQuerier,
    dango_types::{Days, Dimensionless},
    grug::{Order as IterationOrder, Response, SudoCtx},
};

/// Periodic premium sampling and funding collection.
///
/// Called frequently by the chain's cron mechanism (e.g., every minute).
/// Each invocation:
///
/// 1. Samples the current premium for each pair (from impact prices)
///    and accumulates it into `premium_sum` / `premium_samples`.
/// 2. If `funding_period` has elapsed since `last_funding_time`,
///    computes the average premium, applies the funding delta, and
///    resets the accumulators.
///
/// Mutates:
///
/// - `PAIR_STATES[pair_id].premium_sum` (every tick)
/// - `PAIR_STATES[pair_id].premium_samples` (every tick)
/// - `PAIR_STATES[pair_id].funding_per_unit` (at funding collection)
/// - `PAIR_STATES[pair_id].last_funding_time` (at funding collection)
///
/// Returns: an empty response.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let pair_ids = PAIR_IDS.load(ctx.storage)?;
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    for pair_id in &pair_ids {
        let pair_param = PAIR_PARAMS.load(ctx.storage, pair_id)?;
        let mut pair_state = PAIR_STATES.load(ctx.storage, pair_id)?;
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

        // --- Step 1: Sample premium ---

        // Walk the bid side: stored in ascending order of inverted price,
        // so ascending iteration gives best-bid-first. Un-invert to get
        // real prices. Bid order sizes are guaranteed positive.
        let bid_iter = BIDS
            .prefix(pair_id.clone())
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .map(|res| {
                let ((stored_price, _), order) = res?;
                let real_price = may_invert_price(stored_price, true);
                Ok((real_price, order.size))
            });

        // Walk the ask side: stored naturally in ascending price order,
        // so ascending iteration gives best-ask-first.
        let ask_iter = ASKS
            .prefix(pair_id.clone())
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .map(|res| {
                let ((stored_price, _), order) = res?;
                Ok((stored_price, order.size.checked_abs()?))
            });

        let impact_bid = compute_impact_price(bid_iter, pair_param.impact_notional)?;
        let impact_ask = compute_impact_price(ask_iter, pair_param.impact_notional)?;

        let premium = compute_premium(impact_bid, impact_ask, oracle_price)?;

        pair_state.premium_sum.checked_add_assign(premium)?;
        pair_state.premium_samples += 1;

        // --- Step 2: Collect funding if period has elapsed ---

        let elapsed = ctx.block.timestamp - pair_state.last_funding_time;
        if elapsed >= pair_param.funding_period {
            let interval = Days::from_duration(elapsed)?;
            let avg_premium = pair_state
                .premium_sum
                .checked_div(Dimensionless::new_int(pair_state.premium_samples))?;

            let funding_delta = compute_funding_delta(
                avg_premium,
                oracle_price,
                pair_param.max_abs_funding_rate,
                interval,
            )?;

            (pair_state.funding_per_unit).checked_add_assign(funding_delta)?;
            pair_state.last_funding_time = ctx.block.timestamp;

            pair_state.premium_sum = Dimensionless::ZERO;
            pair_state.premium_samples = 0;
        }

        PAIR_STATES.save(ctx.storage, pair_id, &pair_state)?;
    }

    Ok(Response::new())
}
