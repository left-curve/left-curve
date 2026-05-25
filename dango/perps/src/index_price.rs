use {
    crate::{
        MAX_ORACLE_STALENESS,
        core::compute_ewma_index_price,
        state::{PAIR_IDS, PAIR_PARAMS, PAIR_STATES},
    },
    dango_oracle::OracleQuerier,
    dango_order_book::{ASKS, BIDS, PairId, compute_impact_price, may_invert_price},
    dango_types::{
        oracle::Price,
        perps::{PairParam, PairState},
    },
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

        let price = oracle_querier.query_price(&pair_id, None);

        process_index_price_for_pair(
            storage,
            current_time,
            &pair_id,
            &pair_param,
            &mut pair_state,
            price,
        )?;

        PAIR_STATES.save(storage, &pair_id, &pair_state)?;

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                %pair_id,
                index_price = %pair_state.index_price,
                "Updated index price"
            );
        }
    }

    Ok(())
}

fn process_index_price_for_pair(
    storage: &dyn Storage,
    current_time: Timestamp,
    pair_id: &PairId,
    pair_param: &PairParam,
    pair_state: &mut PairState,
    price: anyhow::Result<Price>,
) -> anyhow::Result<()> {
    let index_price = match &price {
        // The oracle is considered available when the market is in regular
        // session (not pre/post/closed) and the price is fresh enough.
        // When available, snap to the oracle price.
        Ok(p)
            if p.market_session == MarketSession::Regular
                && p.timestamp >= current_time - MAX_ORACLE_STALENESS =>
        {
            p.humanized_price
        },
        // When oracle is not available, use EWMA driven by the order book's
        // impact bid/ask spread.
        _ => {
            #[cfg(feature = "tracing")]
            match price {
                Ok(p) => {
                    tracing::warn!(
                        %pair_id,
                        price = %p.humanized_price,
                        market_session = ?p.market_session,
                        timestamp = p.timestamp.to_rfc3339_string(),
                        "Oracle unavailable; using EWMA"
                    );
                },
                Err(err) => {
                    tracing::warn!(
                        %pair_id,
                        %err,
                        "Oracle query failed; using EWMA"
                    );
                },
            }

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

            compute_ewma_index_price(pair_state.index_price, impact_bid, impact_ask, delta_t)?
        },
    };

    pair_state.index_price = index_price;
    pair_state.last_index_time = current_time;

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        anyhow::anyhow,
        dango_order_book::{LimitOrder, OrderKey, Quantity, UsdPrice, UsdValue},
        grug_math::Uint64,
        grug_types::{Addr, Duration, MockStorage},
    };

    const T: Timestamp = Timestamp::from_seconds(1_700_000_000);
    const MAKER: Addr = Addr::mock(99);

    fn btc_pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn default_pair_param() -> PairParam {
        PairParam {
            impact_size: UsdValue::new_int(10_000),
            ..Default::default()
        }
    }

    fn save_bid(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        order_id: u64,
        price: i128,
        size: i128,
    ) {
        let stored_price = may_invert_price(UsdPrice::new_int(price), true);
        let key: OrderKey = (pair_id.clone(), stored_price, Uint64::new(order_id));
        BIDS.save(storage, key, &LimitOrder {
            user: MAKER,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
            created_at: Timestamp::ZERO,
            tp: None,
            sl: None,
            client_order_id: None,
        })
        .unwrap();
    }

    fn save_ask(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        order_id: u64,
        price: i128,
        size: i128,
    ) {
        let key: OrderKey = (
            pair_id.clone(),
            UsdPrice::new_int(price),
            Uint64::new(order_id),
        );
        ASKS.save(storage, key, &LimitOrder {
            user: MAKER,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
            created_at: Timestamp::ZERO,
            tp: None,
            sl: None,
            client_order_id: None,
        })
        .unwrap();
    }

    // ---- Oracle availability decision ----

    /// When the oracle reports a Regular market session and the price timestamp
    /// is within the staleness window, the index price must snap to the oracle
    /// price regardless of the order book state.
    #[test]
    fn a1_regular_fresh_snaps_to_oracle() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price = Ok(Price::new(
            UsdPrice::new_int(105),
            T,
            MarketSession::Regular,
        ));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(105));
        assert_eq!(pair_state.last_index_time, T);
    }

    /// A price whose timestamp is exactly MAX_ORACLE_STALENESS old is still
    /// considered fresh (the comparison is `>=`). Verify the boundary is
    /// inclusive.
    #[test]
    fn a2_staleness_boundary_inclusive() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price = Ok(Price::new(
            UsdPrice::new_int(105),
            T - MAX_ORACLE_STALENESS,
            MarketSession::Regular,
        ));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(105));
    }

    /// A Regular-session price that is 1 ms beyond the staleness window is
    /// treated as unavailable. The index must NOT snap to the oracle; instead
    /// EWMA runs. With an empty book the index stays put.
    #[test]
    fn a3_regular_stale_falls_through_to_ewma() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price = Ok(Price::new(
            UsdPrice::new_int(105),
            T - MAX_ORACLE_STALENESS - Duration::from_millis(1),
            MarketSession::Regular,
        ));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        // Empty book -> IPD=0 -> no movement. Crucially, does NOT snap to 105.
        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    /// When the market session is Other (closed/pre/post), the oracle is
    /// unavailable even if the timestamp is perfectly fresh. The index must
    /// NOT snap to the oracle price.
    #[test]
    fn a4_other_session_uses_ewma_not_snap() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price = Ok(Price::new(UsdPrice::new_int(200), T, MarketSession::Other));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        // Must NOT snap to 200. Empty book -> no movement.
        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    /// When the oracle query itself fails (network error, missing price
    /// source), the EWMA path runs as a graceful fallback.
    #[test]
    fn a5_oracle_query_failure_uses_ewma() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price: anyhow::Result<Price> = Err(anyhow!("connection failed"));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    /// When the market reopens after a period of EWMA drift, the index must
    /// snap back to the oracle price immediately with no smoothing — the
    /// external feed is authoritative.
    #[test]
    fn a6_reopen_snaps_back_from_drift() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(110),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        let price = Ok(Price::new(
            UsdPrice::new_int(102),
            T,
            MarketSession::Regular,
        ));

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            price,
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(102));
    }

    // ---- EWMA with order book ----

    fn other_price() -> anyhow::Result<Price> {
        Ok(Price::new(UsdPrice::new_int(999), T, MarketSession::Other))
    }

    /// When the index sits between the impact bid and impact ask, IPD is zero
    /// and the index does not move — the order book is consistent with the
    /// current price.
    #[test]
    fn b1_index_inside_spread_no_movement() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        // bid@100 with sufficient depth (size 200 @ 100 = 20,000 >= 10,000)
        save_bid(&mut storage, &pair_id, 1, 100, 200);
        // ask@106 with sufficient depth (size 200 @ 106 = 21,200 >= 10,000)
        save_ask(&mut storage, &pair_id, 2, 106, -200);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(103),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(103));
    }

    /// When the index is below the impact bid, the order book's bid side has
    /// moved past the index — buyers are willing to pay more. The index must
    /// drift upward.
    #[test]
    fn b2_index_below_bid_drifts_up() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        save_bid(&mut storage, &pair_id, 1, 100, 200);
        save_ask(&mut storage, &pair_id, 2, 106, -200);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(95),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        assert!(pair_state.index_price > UsdPrice::new_int(95));
    }

    /// When the index is above the impact ask, sellers are willing to sell
    /// below the index. The index must drift downward.
    #[test]
    fn b3_index_above_ask_drifts_down() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        save_bid(&mut storage, &pair_id, 1, 100, 200);
        save_ask(&mut storage, &pair_id, 2, 106, -200);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(112),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        assert!(pair_state.index_price < UsdPrice::new_int(112));
    }

    /// When the bid side has insufficient depth to fill the impact notional,
    /// its contribution to IPD is zero. Only the ask side pulls the index.
    #[test]
    fn b4_bid_insufficient_only_ask_pulls() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        // Tiny bid: size 1 @ 100 = notional 100 < impact_size 10,000
        save_bid(&mut storage, &pair_id, 1, 100, 1);
        // Sufficient ask
        save_ask(&mut storage, &pair_id, 2, 106, -200);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(112),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        // Index above ask, bid has no contribution -> only ask pulls down.
        assert!(pair_state.index_price < UsdPrice::new_int(112));
    }

    /// When the ask side has insufficient depth, its contribution to IPD is
    /// zero. Only the bid side pulls the index.
    #[test]
    fn b5_ask_insufficient_only_bid_pulls() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        // Sufficient bid
        save_bid(&mut storage, &pair_id, 1, 100, 200);
        // Tiny ask: size 1 @ 106 = notional 106 < impact_size 10,000
        save_ask(&mut storage, &pair_id, 2, 106, -1);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(95),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        // Index below bid, ask has no contribution -> only bid pulls up.
        assert!(pair_state.index_price > UsdPrice::new_int(95));
    }

    /// When both sides of the book have insufficient depth, neither side
    /// produces a reliable impact price, so IPD is zero and the index holds.
    #[test]
    fn b6_both_insufficient_no_movement() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        save_bid(&mut storage, &pair_id, 1, 100, 1);
        save_ask(&mut storage, &pair_id, 2, 106, -1);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    /// A completely empty order book is the degenerate case of both sides
    /// having no depth — the index stays put.
    #[test]
    fn b7_empty_book_no_movement() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T - Duration::from_seconds(3),
            ..Default::default()
        };

        process_index_price_for_pair(
            &storage,
            T,
            &pair_id,
            &default_pair_param(),
            &mut pair_state,
            other_price(),
        )
        .unwrap();

        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    // ---- Transition sequences ----

    /// Full close-then-reopen cycle: snap to oracle, drift via EWMA for two
    /// ticks while the market is closed, then snap back to a new oracle price
    /// when the market reopens. The snap-back must be exact — no residual
    /// smoothing from the accumulated drift.
    #[test]
    fn d1_close_then_reopen_snaps_back() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let pair_param = default_pair_param();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(50),
            last_index_time: T - Duration::from_seconds(10),
            ..Default::default()
        };

        // Step 1: Regular@100 -> snap
        let t1 = T;
        let price = Ok(Price::new(
            UsdPrice::new_int(100),
            t1,
            MarketSession::Regular,
        ));
        process_index_price_for_pair(&storage, t1, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();
        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));

        // Place orders: bid@105, ask@110
        save_bid(&mut storage, &pair_id, 1, 105, 200);
        save_ask(&mut storage, &pair_id, 2, 110, -200);

        // Step 2: Other, 3s later -> EWMA drift up
        let t2 = t1 + Duration::from_seconds(3);
        let price = Ok(Price::new(UsdPrice::new_int(999), t2, MarketSession::Other));
        process_index_price_for_pair(&storage, t2, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();
        let after_step2 = pair_state.index_price;
        assert!(after_step2 > UsdPrice::new_int(100));

        // Step 3: Other, 3s later -> further drift
        let t3 = t2 + Duration::from_seconds(3);
        let price = Ok(Price::new(UsdPrice::new_int(999), t3, MarketSession::Other));
        process_index_price_for_pair(&storage, t3, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();
        assert!(pair_state.index_price > after_step2);

        // Step 4: Regular@98 -> snap back exactly
        let t4 = t3 + Duration::from_seconds(3);
        let price = Ok(Price::new(
            UsdPrice::new_int(98),
            t4,
            MarketSession::Regular,
        ));
        process_index_price_for_pair(&storage, t4, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();
        assert_eq!(pair_state.index_price, UsdPrice::new_int(98));
    }

    /// A Regular-session price becomes stale when the block timestamp advances
    /// past the staleness window without a new oracle feed. The second call
    /// must fall through to EWMA even though the session is still Regular.
    #[test]
    fn d2_regular_goes_stale() {
        let storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let pair_param = default_pair_param();
        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(50),
            last_index_time: T - Duration::from_seconds(10),
            ..Default::default()
        };

        // Step 1: Regular fresh@100 -> snap
        let t1 = T;
        let price = Ok(Price::new(
            UsdPrice::new_int(100),
            t1,
            MarketSession::Regular,
        ));
        process_index_price_for_pair(&storage, t1, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();
        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));

        // Step 2: Same price object but current_time advances past staleness.
        // Timestamp t1 is now stale relative to t2.
        let t2 = t1 + Duration::from_seconds(1);
        let price = Ok(Price::new(
            UsdPrice::new_int(100),
            t1,
            MarketSession::Regular,
        ));
        process_index_price_for_pair(&storage, t2, &pair_id, &pair_param, &mut pair_state, price)
            .unwrap();

        // Empty book, EWMA with no depth -> holds at 100.
        assert_eq!(pair_state.index_price, UsdPrice::new_int(100));
    }

    /// Five consecutive EWMA ticks with the index below the impact bid.
    /// Each tick's output feeds into the next as input. The drift must be
    /// monotonically increasing (toward the bid) and each per-tick increment
    /// must be smaller than the previous one — because the IPD shrinks as
    /// the index approaches the bid.
    #[test]
    fn d3_accumulated_drift_monotonic_decreasing_increments() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let pair_param = default_pair_param();

        // bid@110, ask@115
        save_bid(&mut storage, &pair_id, 1, 110, 200);
        save_ask(&mut storage, &pair_id, 2, 115, -200);

        let mut pair_state = PairState {
            index_price: UsdPrice::new_int(100),
            last_index_time: T,
            ..Default::default()
        };

        let mut prev_index = pair_state.index_price;
        let mut prev_increment = UsdPrice::new_int(999);

        for i in 1..=5u128 {
            let t = T + Duration::from_seconds(i * 3);
            process_index_price_for_pair(
                &storage,
                t,
                &pair_id,
                &pair_param,
                &mut pair_state,
                other_price(),
            )
            .unwrap();

            let current = pair_state.index_price;
            let increment = current.checked_sub(prev_index).unwrap();

            // Monotonically increasing toward bid (110)
            assert!(
                current > prev_index,
                "tick {i}: expected monotonic increase"
            );

            // Each increment should be smaller than the previous (IPD shrinks)
            if i > 1 {
                assert!(
                    increment < prev_increment,
                    "tick {i}: expected decreasing increments, got {increment} >= {prev_increment}"
                );
            }

            prev_index = current;
            prev_increment = increment;
        }

        // After 5 ticks of 3s, should still be below 110 (the bid)
        assert!(pair_state.index_price < UsdPrice::new_int(110));
    }
}
