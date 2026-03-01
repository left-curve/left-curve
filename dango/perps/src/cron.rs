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

    let impact_bid = compute_impact_price(bid_iter, pair_param.impact_size)?;
    let impact_ask = compute_impact_price(ask_iter, pair_param.impact_size)?;

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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            FundingPerUnit, FundingRate, Quantity,
            oracle::PrecisionedPrice,
            perps::{Order, PairParam, PairState, Param, State, Unlock},
        },
        grug::{Duration, MockStorage, NumberConst, Udec128, Uint64, hash_map},
        std::collections::{BTreeSet, VecDeque},
    };

    const USER_A: Addr = Addr::mock(1);
    const USER_B: Addr = Addr::mock(2);
    const MAKER: Addr = Addr::mock(3);

    fn btc_pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn eth_pair_id() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    /// Build unlocks from `(usd_amount, end_time_seconds)` pairs.
    ///
    /// Mutates: nothing.
    /// Returns: a `VecDeque<Unlock>` for use in `UserState`.
    fn unlocks_from(entries: &[(i128, u128)]) -> VecDeque<Unlock> {
        entries
            .iter()
            .map(|&(amount, secs)| Unlock {
                amount_to_release: UsdValue::new_int(amount),
                end_time: Timestamp::from_seconds(secs),
            })
            .collect()
    }

    /// Mock oracle returning the settlement currency (USDC) at $1.
    ///
    /// Mutates: nothing.
    /// Returns: an `OracleQuerier` suitable for `process_unlocks` tests.
    fn settlement_oracle() -> OracleQuerier<'static> {
        OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::ONE,
                Timestamp::from_seconds(0),
                6,
            ),
        })
    }

    /// Place a resting bid order into `BIDS` storage.
    ///
    /// Mutates: writes to `storage`.
    /// Returns: nothing.
    fn place_bid_order(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        price: i128,
        size: i128,
        order_id: u64,
    ) {
        let inverted_price = !UsdPrice::new_int(price);
        let key = (pair_id.clone(), inverted_price, Uint64::new(order_id));
        let order = Order {
            user: MAKER,
            size: Quantity::new_int(size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
        };
        BIDS.save(storage, key, &order).unwrap();
    }

    /// Place a resting ask order into `ASKS` storage.
    ///
    /// Mutates: writes to `storage`.
    /// Returns: nothing.
    fn place_ask_order(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        price: i128,
        size: i128,
        order_id: u64,
    ) {
        let key = (
            pair_id.clone(),
            UsdPrice::new_int(price),
            Uint64::new(order_id),
        );
        let order = Order {
            user: MAKER,
            size: Quantity::new_int(-size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
        };
        ASKS.save(storage, key, &order).unwrap();
    }

    /// Save the common funding-related storage items for a single pair.
    ///
    /// Mutates: writes `PARAM`, `STATE`, `PAIR_IDS`, `PAIR_PARAMS`, `PAIR_STATES`
    /// to `storage`.
    /// Returns: nothing.
    fn init_funding_storage(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        pair_param: &PairParam,
        pair_state: &PairState,
        funding_period_secs: u128,
        last_funding_time_secs: u128,
    ) {
        PARAM
            .save(storage, &Param {
                funding_period: Duration::from_seconds(funding_period_secs),
                ..Default::default()
            })
            .unwrap();
        STATE
            .save(storage, &State {
                last_funding_time: Timestamp::from_seconds(last_funding_time_secs),
                ..Default::default()
            })
            .unwrap();
        PAIR_IDS
            .save(storage, &BTreeSet::from([pair_id.clone()]))
            .unwrap();
        PAIR_PARAMS.save(storage, pair_id, pair_param).unwrap();
        PAIR_STATES.save(storage, pair_id, pair_state).unwrap();
    }

    fn default_funding_pair_param() -> PairParam {
        PairParam {
            impact_size: UsdValue::new_int(10_000),
            max_abs_funding_rate: FundingRate::new_raw(50_000), // 0.05/day
            ..Default::default()
        }
    }

    // ==================== process_unlocks tests ====================

    #[test]
    fn no_matured_unlocks_returns_none() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 200), (2000, 300)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(100), &mut oracle).unwrap();
        assert!(result.is_none());

        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.unlocks.len(), 2);
    }

    #[test]
    fn single_user_single_matured_unlock() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 100)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        // At t=100 the unlock matures (end_time > current_time is false).
        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(100), &mut oracle).unwrap();
        assert!(result.is_some());

        // Only state was the single unlock → user removed.
        assert!(USER_STATES.may_load(&storage, USER_A).unwrap().is_none());
    }

    #[test]
    fn single_user_partial_maturation() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 100), (2000, 200), (3000, 300)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        // At t=200 the first two unlocks mature.
        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(200), &mut oracle).unwrap();
        assert!(result.is_some());

        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.unlocks.len(), 1);
        assert_eq!(loaded.unlocks[0].amount_to_release, UsdValue::new_int(3000));
    }

    #[test]
    fn multiple_users_batched_transfer() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        USER_STATES
            .save(&mut storage, USER_A, &UserState {
                unlocks: unlocks_from(&[(500, 50)]),
                ..Default::default()
            })
            .unwrap();
        USER_STATES
            .save(&mut storage, USER_B, &UserState {
                unlocks: unlocks_from(&[(700, 60)]),
                ..Default::default()
            })
            .unwrap();

        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(100), &mut oracle).unwrap();
        assert!(result.is_some());

        // Both user states removed.
        assert!(USER_STATES.may_load(&storage, USER_A).unwrap().is_none());
        assert!(USER_STATES.may_load(&storage, USER_B).unwrap().is_none());
    }

    #[test]
    fn user_with_margin_preserved_after_unlock() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        // User has unlocks AND nonzero margin.
        let user_state = UserState {
            margin: UsdValue::new_int(500),
            unlocks: unlocks_from(&[(1000, 100)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(200), &mut oracle).unwrap();
        assert!(result.is_some());

        // User state persists because margin keeps is_empty() false.
        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.margin, UsdValue::new_int(500));
        assert!(loaded.unlocks.is_empty());
    }

    #[test]
    fn no_users_returns_none() {
        let mut storage = MockStorage::new();
        let mut oracle = settlement_oracle();

        let result =
            process_unlocks(&mut storage, Timestamp::from_seconds(100), &mut oracle).unwrap();
        assert!(result.is_none());
    }

    // ==================== process_funding tests ====================

    #[test]
    fn funding_skipped_when_period_not_elapsed() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        // Only 1800s elapsed, period is 3600s → funding skipped.
        process_funding(&mut storage, Timestamp::from_seconds(1800), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(0));

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_eq!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_applied_when_period_elapsed() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        // Bid at $51,000 above oracle $50,000 → positive premium.
        place_bid_order(&mut storage, &pair_id, 51_000, 1, 1);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_ne!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_with_empty_order_book() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        // Empty book → premium = 0 → delta = 0.
        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_eq!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_multiple_pairs() {
        let mut storage = MockStorage::new();
        let btc = btc_pair_id();
        let eth = eth_pair_id();
        let pair_param = default_funding_pair_param();

        PARAM
            .save(&mut storage, &Param {
                funding_period: Duration::from_seconds(3600),
                ..Default::default()
            })
            .unwrap();
        STATE
            .save(&mut storage, &State {
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            })
            .unwrap();
        PAIR_IDS
            .save(&mut storage, &BTreeSet::from([btc.clone(), eth.clone()]))
            .unwrap();
        PAIR_PARAMS.save(&mut storage, &btc, &pair_param).unwrap();
        PAIR_PARAMS.save(&mut storage, &eth, &pair_param).unwrap();
        PAIR_STATES
            .save(&mut storage, &btc, &PairState::default())
            .unwrap();
        PAIR_STATES
            .save(&mut storage, &eth, &PairState::default())
            .unwrap();

        // BTC: bid above oracle → positive premium.
        place_bid_order(&mut storage, &btc, 51_000, 1, 1);
        // ETH: ask below oracle → negative premium.
        place_ask_order(&mut storage, &eth, 2_900, 10, 2);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            btc.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
            eth.clone() => PrecisionedPrice::new(
                Udec128::new_percent(300_000), // $3,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        let btc_state = PAIR_STATES.load(&storage, &btc).unwrap();
        assert_ne!(btc_state.funding_per_unit, FundingPerUnit::ZERO);

        let eth_state = PAIR_STATES.load(&storage, &eth).unwrap();
        assert_ne!(eth_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_updates_accumulator_not_replaces() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let initial_funding = FundingPerUnit::new_int(100);

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState {
                funding_per_unit: initial_funding,
                ..Default::default()
            },
            3600,
            0,
        );

        // Bid above oracle → positive delta added to existing accumulator.
        place_bid_order(&mut storage, &pair_id, 51_000, 1, 1);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        // Accumulator = initial (100) + positive delta, so strictly greater.
        assert!(pair_state.funding_per_unit > initial_funding);
    }
}
