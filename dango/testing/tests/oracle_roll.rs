use {
    dango_order_book::{Dimensionless, UsdPrice},
    dango_testing::setup_test_naive,
    dango_types::oracle::{
        ExecuteMsg, Fixing, PriceConfig, PriceSource, QueryPriceRequest, QueryPriceSourceRequest,
        RollScheduleUpdate, RollState, ScheduledRoll,
    },
    grug_types::{Coins, Denom, Duration, QuerierExt, ResultExt, Timestamp, btree_map},
    pyth_types::{Channel, MarketSession},
    std::{collections::VecDeque, str::FromStr},
};

fn source(id: u32) -> PriceSource {
    PriceSource {
        id,
        channel: Channel::RealTime,
    }
}

/// A futures-roll denom blends `current` and `next` by the weight in force at
/// the block timestamp: 100% `current` before the first fixing, the weighted
/// mean in between, 100% `next` after the last fixing.
#[tokio::test]
async fn roll_price_blends_over_the_schedule() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let oracle = contracts.oracle;
    suite.block_time = Duration::from_seconds(1);

    let denom = Denom::from_str("perp/oil").unwrap();
    let t0 = suite.block.timestamp;

    // 100% current until t0+100, 40% on next from t0+100, 100% next from t0+200.
    let roll = PriceConfig::Roll(RollState {
        current: source(9001),
        next: source(9002),
        fixings: vec![
            Fixing {
                at: t0 + Duration::from_seconds(100),
                next_weight: Dimensionless::new_percent(40),
            },
            Fixing {
                at: t0 + Duration::from_seconds(200),
                next_weight: Dimensionless::ONE,
            },
        ],
        upcoming: VecDeque::new(),
    });
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! { denom.clone() => roll }),
            Coins::new(),
        )
        .await
        .should_succeed();

    // Feed both contracts: current = $100, next = $200.
    suite
        .do_oracle_actions(
            &mut accounts.owner,
            None,
            Some((
                &[
                    (9001, UsdPrice::new_int(100), MarketSession::Regular),
                    (9002, UsdPrice::new_int(200), MarketSession::Regular),
                ],
                Timestamp::MAX,
            )),
            false,
            false,
        )
        .await;

    // Before the first fixing: 100% current = $100.
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: denom.clone(),
        })
        .should_succeed();
    assert_eq!(price.humanized_price, UsdPrice::new_int(100));

    // Into the 40% window (~t0+152): 100*0.6 + 200*0.4 = $140.
    suite.increase_time(Duration::from_seconds(150)).await;
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: denom.clone(),
        })
        .should_succeed();
    assert_eq!(price.humanized_price, UsdPrice::new_int(140));

    // After the last fixing (~t0+252): 100% next = $200.
    suite.increase_time(Duration::from_seconds(100)).await;
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: denom.clone(),
        })
        .should_succeed();
    assert_eq!(price.humanized_price, UsdPrice::new_int(200));
}

/// The maintenance cron advances a roll once its last fixing has passed:
/// `next` becomes `current` and the head of `upcoming` becomes the new `next`.
#[tokio::test]
async fn cron_advances_completed_roll() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let oracle = contracts.oracle;
    suite.block_time = Duration::from_seconds(1);

    let denom = Denom::from_str("perp/oil").unwrap();
    let t0 = suite.block.timestamp;

    let roll = PriceConfig::Roll(RollState {
        current: source(9001),
        next: source(9002),
        fixings: vec![Fixing {
            at: t0 + Duration::from_seconds(10),
            next_weight: Dimensionless::ONE,
        }],
        upcoming: VecDeque::from([ScheduledRoll {
            contract: source(9003),
            fixings: vec![Fixing {
                at: t0 + Duration::from_seconds(10_000),
                next_weight: Dimensionless::ONE,
            }],
        }]),
    });
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! { denom.clone() => roll }),
            Coins::new(),
        )
        .await
        .should_succeed();

    // Not advanced yet.
    let PriceConfig::Roll(before) = suite
        .query_wasm_smart(oracle, QueryPriceSourceRequest {
            denom: denom.clone(),
        })
        .should_succeed()
    else {
        panic!("expected a roll");
    };
    assert_eq!(before.current.id, 9001);

    // Advance past the last fixing (10s) and the cron interval (60s): the cron
    // fires during the block and advances the roll.
    suite.increase_time(Duration::from_seconds(120)).await;

    let PriceConfig::Roll(after) = suite
        .query_wasm_smart(oracle, QueryPriceSourceRequest {
            denom: denom.clone(),
        })
        .should_succeed()
    else {
        panic!("expected a roll");
    };
    assert_eq!(after.current.id, 9002, "next was promoted to current");
    assert_eq!(after.next.id, 9003, "successor pulled from upcoming");
    assert!(after.upcoming.is_empty());
}

/// `UpdateRollSchedules` is owner-only, appends to / overrides the `upcoming`
/// queue, and re-validates the whole config so a bad schedule is rejected.
#[tokio::test]
async fn update_roll_schedules_append_override_and_auth() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());
    let oracle = contracts.oracle;
    suite.block_time = Duration::from_seconds(1);

    let denom = Denom::from_str("perp/oil").unwrap();
    let t0 = suite.block.timestamp;

    let roll = PriceConfig::Roll(RollState {
        current: source(9001),
        next: source(9002),
        fixings: vec![Fixing {
            at: t0 + Duration::from_seconds(100),
            next_weight: Dimensionless::ONE,
        }],
        upcoming: VecDeque::new(),
    });
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! { denom.clone() => roll }),
            Coins::new(),
        )
        .await
        .should_succeed();

    let scheduled = |id: u32, at_secs: u128| ScheduledRoll {
        contract: source(id),
        fixings: vec![Fixing {
            at: t0 + Duration::from_seconds(at_secs),
            next_weight: Dimensionless::ONE,
        }],
    };

    // A non-owner cannot update schedules.
    suite
        .execute(
            &mut accounts.user1,
            oracle,
            &ExecuteMsg::UpdateRollSchedules(btree_map! {
                denom.clone() => RollScheduleUpdate::Append(vec![scheduled(9003, 200)]),
            }),
            Coins::new(),
        )
        .await
        .should_fail_with_error("don't have the right");

    // The owner appends a valid scheduled roll.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::UpdateRollSchedules(btree_map! {
                denom.clone() => RollScheduleUpdate::Append(vec![scheduled(9003, 200)]),
            }),
            Coins::new(),
        )
        .await
        .should_succeed();
    let PriceConfig::Roll(r) = suite
        .query_wasm_smart(oracle, QueryPriceSourceRequest {
            denom: denom.clone(),
        })
        .should_succeed()
    else {
        panic!("expected a roll");
    };
    assert_eq!(r.upcoming.len(), 1);
    assert_eq!(r.upcoming[0].contract.id, 9003);

    // Appending an out-of-order roll (starts before the previous one's last
    // fixing at t0+200) is rejected by validation.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::UpdateRollSchedules(btree_map! {
                denom.clone() => RollScheduleUpdate::Append(vec![scheduled(9004, 150)]),
            }),
            Coins::new(),
        )
        .await
        .should_fail_with_error("after the previous");

    // Override replaces the whole queue.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::UpdateRollSchedules(btree_map! {
                denom.clone() => RollScheduleUpdate::Override(vec![scheduled(9005, 300)]),
            }),
            Coins::new(),
        )
        .await
        .should_succeed();
    let PriceConfig::Roll(r) = suite
        .query_wasm_smart(oracle, QueryPriceSourceRequest {
            denom: denom.clone(),
        })
        .should_succeed()
    else {
        panic!("expected a roll");
    };
    assert_eq!(r.upcoming.len(), 1);
    assert_eq!(r.upcoming[0].contract.id, 9005);
}
