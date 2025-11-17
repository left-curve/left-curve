use {
    grug_testing::TestBuilder,
    grug_types::{
        Binary, Coin, Coins, Duration, Empty, QuerierExt, ResultExt, Timestamp, btree_map,
    },
    grug_vm_rust::ContractBuilder,
};

/// A contract that implements the `cron_execute` export function. Used for
/// testing whether the app can correctly handle cronjobs.
///
/// The specific job it's going to do, is to send a predefined amount of coin to
/// a predefined receiver address.
mod tester {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        grug_storage::Item,
        grug_types::{Addr, Coin, Message, MutableCtx, Response, StdResult, SudoCtx},
        serde::{Deserialize, Serialize},
    };

    const JOB: Item<Job> = Item::new("job");

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    pub struct Job {
        pub receiver: Addr,
        pub coin: Coin,
    }

    pub fn instantiate(ctx: MutableCtx, job: Job) -> StdResult<Response> {
        JOB.save(ctx.storage, &job).unwrap();

        Ok(Response::new())
    }

    pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
        let job = JOB.load(ctx.storage).unwrap();

        Ok(Response::new().add_message(Message::transfer(job.receiver, job.coin).unwrap()))
    }
}

/// A cronjob contract that intentionally fails during `cron_execute`. Used for
/// testing whether the app can correctly handle revert failing cronjobs state changes.
mod failing_tester {
    use {
        grug_math::{Number, NumberConst, Uint128},
        grug_types::{Empty, MutableCtx, Response, StdResult, SudoCtx},
    };

    pub fn instantiate(ctx: MutableCtx, _: Empty) -> StdResult<Response> {
        ctx.storage.write(b"foo", b"init");

        Ok(Response::new())
    }

    pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
        ctx.storage.write(b"foo", b"cron_execute");

        // This should fail.
        let _ = Uint128::ONE.checked_div(Uint128::ZERO)?;

        Ok(Response::new())
    }
}

struct Balances {
    uatom: u128,
    uosmo: u128,
    umars: u128,
}

#[test]
fn cronjob_works() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("larry", [("uatom", 100), ("uosmo", 100), ("umars", 100)])
        .add_account("jake", Coins::new())
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("larry")
        .build();

    let tester_code = ContractBuilder::new(Box::new(tester::instantiate))
        .with_cron_execute(Box::new(tester::cron_execute))
        .build();

    let receiver = accounts["jake"].address;

    // Block time: 1
    //
    // Upload the tester contract code.
    let tester_code_hash = suite
        .upload(&mut accounts["larry"], tester_code)
        .should_succeed()
        .code_hash;

    // Block time: 2
    //
    // Deploy three tester contracts with different jobs.
    // Each contract is given an initial coin balance.
    let cron1 = suite
        .instantiate(
            &mut accounts["larry"],
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new("uatom", 1).unwrap(),
            },
            "cron1",
            Some("cron1"),
            None,
            Coins::one("uatom", 3).unwrap(),
        )
        .should_succeed()
        .address;

    // Block time: 3
    let cron2 = suite
        .instantiate(
            &mut accounts["larry"],
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new("uosmo", 1).unwrap(),
            },
            "cron2",
            Some("cron2"),
            None,
            Coins::one("uosmo", 3).unwrap(),
        )
        .should_succeed()
        .address;

    // Block time: 4
    let cron3 = suite
        .instantiate(
            &mut accounts["larry"],
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new("umars", 1).unwrap(),
            },
            "cron3",
            Some("cron3"),
            None,
            Coins::one("umars", 3).unwrap(),
        )
        .should_succeed()
        .address;

    // Block time: 5
    //
    // Schedule a chain upgrade. The chain upgrade will set the cronjobs.
    suite
        .upgrade(
            &mut accounts["larry"],
            6,
            "0.1.0",
            None::<&str>,
            None::<&str>,
        )
        .should_succeed();

    let mut new_cfg = suite.query_config().unwrap();
    new_cfg.cronjobs = btree_map! {
        // cron1 has interval of 0, meaning it's to be called every block.
        cron1 => Duration::from_seconds(0),
        cron2 => Duration::from_seconds(2),
        cron3 => Duration::from_seconds(3),
    };

    suite.app.set_cfg(new_cfg);
    suite.app.set_cargo_version_and_upgrade_handler(
        "0.1.0",
        Some(|mut storage, _, block, _, cfg, _| {
            for (contract, interval) in cfg.cronjobs {
                grug_app::schedule_cronjob(&mut storage, contract, block.timestamp + interval)?;
            }

            Ok(())
        }),
    );

    // Make some blocks.
    // After each block, check that Jake has the correct balances.
    for balances in [
        // Block time: 6
        //
        // Chain upgrade runs. Cronjobs are scheduled:
        //
        // cron1 scheduled at 6
        // cron2 scheduled at 8
        // cron3 scheduled at 9
        //
        // cron1 sends 1 uatom, rescheduled to 7
        Balances {
            uatom: 1,
            uosmo: 0,
            umars: 0,
        },
        // Block time: 7
        //
        // cron1 sends 1 uatom, rescheduled to 8
        Balances {
            uatom: 2,
            uosmo: 0,
            umars: 0,
        },
        // Block time: 8
        //
        // cron1 sends 1 uatom, rescheduled to 9 (it runs out of coins here)
        // cron2 sends 1 uosmo, rescheduled to 10
        Balances {
            uatom: 3,
            uosmo: 1,
            umars: 0,
        },
        // Block time: 9
        //
        // cron1 errors because it's out of coins
        // cron3 sends 1 umars, rescheduled to 12
        Balances {
            uatom: 3,
            uosmo: 1,
            umars: 1,
        },
        // Block time: 10
        //
        // cron2 sends 1 uosmo, rescheduled to 12
        Balances {
            uatom: 3,
            uosmo: 2,
            umars: 1,
        },
        // Block time: 11
        //
        // Nothing happens
        Balances {
            uatom: 3,
            uosmo: 2,
            umars: 1,
        },
        // Block time: 12
        //
        // cron2 sends 1 uosmo (runs out of coins), rescheduled to 14
        // cron3 sends 1 umars, rescheduled to 15
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 13
        //
        // Nothing happens
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 14
        //
        // cron2 errors, otherwise nothing happens
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 15
        //
        // cron3 sends 1 umars, runs out of coins
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 3,
        },
    ] {
        // Advance block
        suite.make_empty_block();

        // The balances Jake is expected to have at time point
        let mut expect = Coins::new();
        expect.insert(("uatom", balances.uatom)).unwrap();
        expect.insert(("uosmo", balances.uosmo)).unwrap();
        expect.insert(("umars", balances.umars)).unwrap();

        // Check the balances are correct
        suite
            .query_balances(&accounts["jake"])
            .should_succeed_and_equal(expect);
    }
}

#[test]
fn cronjob_fails() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("larry", Coins::new())
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("larry")
        .build();

    // Block height: 1
    let tester_code = ContractBuilder::new(Box::new(failing_tester::instantiate))
        .with_cron_execute(Box::new(failing_tester::cron_execute))
        .build();

    let tester_code_hash = suite
        .upload(&mut accounts["larry"], tester_code)
        .should_succeed()
        .code_hash;

    // Block height: 2
    let cron = suite
        .instantiate(
            &mut accounts["larry"],
            tester_code_hash,
            &Empty {},
            "cron1",
            Some("cron1"),
            None,
            Coins::default(),
        )
        .should_succeed()
        .address;

    // Block height: 3
    suite
        .upgrade(
            &mut accounts["larry"],
            4,
            "0.1.0",
            None::<&str>,
            None::<&str>,
        )
        .should_succeed();

    let mut new_cfg = suite.query_config().unwrap();
    new_cfg.cronjobs = btree_map! {
        // cron1 has interval of 0, meaning it's to be called every block.
        cron => Duration::from_seconds(0),
    };

    suite.app.set_cfg(new_cfg);
    suite.app.set_cargo_version_and_upgrade_handler(
        "0.1.0",
        Some(|mut storage, _, block, _, cfg, _| {
            for (contract, interval) in cfg.cronjobs {
                grug_app::schedule_cronjob(&mut storage, contract, block.timestamp + interval)?;
            }

            Ok(())
        }),
    );

    // Before the block, storage key `b"foo"` should have the value `b"init"`.
    suite
        .query_wasm_raw(cron, *b"foo")
        .should_succeed_and_equal(Some(Binary::from(*b"init")));

    // Advance block to height 4. Upgrade runs. Cronjob scheduled at the same
    // block. Cronjob runs.
    let res = suite.make_empty_block().block_outcome;
    assert_eq!(res.cron_outcomes.len(), 1);

    // The cronjob attempts to overwrite the value with `b"cron_execute"`.
    // But it then fails before returning, so the change it discarded.
    suite
        .query_wasm_raw(cron, *b"foo")
        .should_succeed_and_equal(Some(Binary::from(*b"init")));
}
