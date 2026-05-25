use {
    crate::{
        MAX_ORACLE_STALENESS,
        core::compute_ewma_index_price,
        state::{PAIR_IDS, PAIR_PARAMS, PAIR_STATES},
    },
    dango_oracle::OracleQuerier,
    dango_order_book::{ASKS, BIDS, compute_impact_price, may_invert_price},
    grug_types::{Order as IterationOrder, Storage, Timestamp},
    pyth_types::MarketSession,
};

/// Update `PairState::index_price` for every active pair.
///
/// When the oracle price is available (regular market session, fresh
/// timestamp), the index price snaps to the oracle price. When it is
/// unavailable (market closed, stale feed), the index price drifts via
/// the EWMA mechanism driven by impact bid/ask from the order book.
pub fn process_index_price(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    let pair_ids = PAIR_IDS.load(storage)?;

    for pair_id in pair_ids {
        let pair_param = PAIR_PARAMS.load(storage, &pair_id)?;
        let mut pair_state = PAIR_STATES.load(storage, &pair_id)?;

        let Ok(price) = oracle_querier.query_price(&pair_id, None) else {
            continue;
        };

        let oracle_available = price.market_session == MarketSession::Regular
            && price.timestamp >= current_time - MAX_ORACLE_STALENESS;

        if oracle_available {
            pair_state.index_price = price.humanized_price;
        } else {
            let bid_iter = BIDS
                .prefix(pair_id.clone())
                .range(storage, None, None, IterationOrder::Ascending)
                .map(|res| {
                    let ((stored_price, _), order) = res?;
                    let real_price = may_invert_price(stored_price, true);
                    Ok((real_price, order.size))
                });

            let ask_iter = ASKS
                .prefix(pair_id.clone())
                .range(storage, None, None, IterationOrder::Ascending)
                .map(|res| {
                    let ((stored_price, _), order) = res?;
                    Ok((stored_price, order.size.checked_abs()?))
                });

            let impact_bid = compute_impact_price(bid_iter, pair_param.impact_size)?;
            let impact_ask = compute_impact_price(ask_iter, pair_param.impact_size)?;

            let delta_t = current_time - pair_state.last_index_time;

            pair_state.index_price =
                compute_ewma_index_price(pair_state.index_price, impact_bid, impact_ask, delta_t)?;
        }

        pair_state.last_index_time = current_time;

        PAIR_STATES.save(storage, &pair_id, &pair_state)?;

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                %pair_id,
                index_price = %pair_state.index_price,
                oracle_available,
                "Updated index price"
            );
        }
    }

    Ok(())
}
