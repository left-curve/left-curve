use {
    dango_testing::{ContractBuilder, TestOption, setup_test_naive},
    dango_types::constants::{dango, eth, usdc},
    grug_types::{
        Addressable, Binary, Coin, Coins, Duration, Empty, Json, QuerierExt, ResultExt, btree_map,
        coins,
    },
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
    usdc: u128,
    eth: u128,
    dango: u128,
}

#[tokio::test]
async fn cronjob_works() {
    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption {
        block_time: Duration::from_seconds(1),
        ..TestOption::default()
    });

    let tester_code = ContractBuilder::new(Box::new(tester::instantiate))
        .with_cron_execute(Box::new(tester::cron_execute))
        .build();

    let receiver = accounts.user1.address();

    // Upload the tester contract code.
    let tester_code_hash = suite
        .upload(&mut accounts.owner, tester_code)
        .await
        .should_succeed()
        .code_hash;

    // Deploy three tester contracts with different jobs.
    // Each contract sends 1 unit of a different denom per cron invocation.
    // Each contract is given 3 of its respective denom.
    let cron1 = suite
        .instantiate(
            &mut accounts.owner,
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new(usdc::DENOM.clone(), 1).unwrap(),
            },
            "cron1",
            Some("cron1"),
            None,
            coins! { usdc::DENOM.clone() => 3 },
        )
        .await
        .should_succeed()
        .address;

    let cron2 = suite
        .instantiate(
            &mut accounts.owner,
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new(eth::DENOM.clone(), 1).unwrap(),
            },
            "cron2",
            Some("cron2"),
            None,
            coins! { eth::DENOM.clone() => 3 },
        )
        .await
        .should_succeed()
        .address;

    let cron3 = suite
        .instantiate(
            &mut accounts.owner,
            tester_code_hash,
            &tester::Job {
                receiver,
                coin: Coin::new(dango::DENOM.clone(), 1).unwrap(),
            },
            "cron3",
            Some("cron3"),
            None,
            coins! { dango::DENOM.clone() => 3 },
        )
        .await
        .should_succeed()
        .address;

    // Update the config to add the cronjobs.
    let mut new_cfg = suite.query_config().unwrap();
    new_cfg.cronjobs = btree_map! {
        // cron1 has interval of 0, meaning it's to be called every block.
        cron1 => Duration::from_seconds(0),
        cron2 => Duration::from_seconds(2),
        cron3 => Duration::from_seconds(3),
    };

    // cron1 scheduled at T (fires on configure block)
    // cron2 scheduled at T+2
    // cron3 scheduled at T+3
    suite
        .configure::<Json>(&mut accounts.owner, Some(new_cfg), None)
        .await
        .should_succeed();

    // Record the receiver's initial balances (after configure block's cron1 fired).
    let initial_usdc = suite
        .query_balance(&accounts.user1, usdc::DENOM.clone())
        .unwrap();
    let initial_eth = suite
        .query_balance(&accounts.user1, eth::DENOM.clone())
        .unwrap();
    let initial_dango = suite
        .query_balance(&accounts.user1, dango::DENOM.clone())
        .unwrap();

    // Make some blocks.
    // After each block, check that the receiver has the correct cumulative
    // increases relative to the initial balances.
    //
    // NOTE: initial_balance is captured AFTER the configure block, during which
    // cron1 already fired once. So cron1's contract has 2 coins remaining.
    //
    // Scheduling:
    // - cron1 (usdc, interval 0): fires every block, scheduled at T
    // - cron2 (eth, interval 2): scheduled at T+2
    // - cron3 (dango, interval 3): scheduled at T+3
    //
    // After configure, cron1 has 2 remaining, cron2 has 3, cron3 has 3.
    for balances in [
        // Empty block 1 (time T+1):
        //
        // cron1 sends 1 usdc (1 remaining after)
        Balances {
            usdc: 1,
            eth: 0,
            dango: 0,
        },
        // Empty block 2 (time T+2):
        //
        // cron1 sends 1 usdc (0 remaining, runs out here)
        // cron2 sends 1 eth (2 remaining), rescheduled to T+4
        Balances {
            usdc: 2,
            eth: 1,
            dango: 0,
        },
        // Empty block 3 (time T+3):
        //
        // cron1 errors (out of coins)
        // cron3 sends 1 dango (2 remaining), rescheduled to T+6
        Balances {
            usdc: 2,
            eth: 1,
            dango: 1,
        },
        // Empty block 4 (time T+4):
        //
        // cron2 sends 1 eth (1 remaining), rescheduled to T+6
        Balances {
            usdc: 2,
            eth: 2,
            dango: 1,
        },
        // Empty block 5 (time T+5):
        //
        // Nothing happens
        Balances {
            usdc: 2,
            eth: 2,
            dango: 1,
        },
        // Empty block 6 (time T+6):
        //
        // cron2 sends 1 eth (0 remaining, runs out), rescheduled to T+8
        // cron3 sends 1 dango (1 remaining), rescheduled to T+9
        Balances {
            usdc: 2,
            eth: 3,
            dango: 2,
        },
        // Empty block 7 (time T+7):
        //
        // Nothing happens
        Balances {
            usdc: 2,
            eth: 3,
            dango: 2,
        },
        // Empty block 8 (time T+8):
        //
        // cron2 errors (out of coins)
        Balances {
            usdc: 2,
            eth: 3,
            dango: 2,
        },
        // Empty block 9 (time T+9):
        //
        // cron3 sends 1 dango (0 remaining, runs out)
        Balances {
            usdc: 2,
            eth: 3,
            dango: 3,
        },
    ] {
        // Advance block
        suite.make_empty_block().await;

        // Check balances
        suite
            .query_balance(&accounts.user1, usdc::DENOM.clone())
            .should_succeed_and_equal(initial_usdc + balances.usdc.into());
        suite
            .query_balance(&accounts.user1, eth::DENOM.clone())
            .should_succeed_and_equal(initial_eth + balances.eth.into());
        suite
            .query_balance(&accounts.user1, dango::DENOM.clone())
            .should_succeed_and_equal(initial_dango + balances.dango.into());
    }
}

#[tokio::test]
async fn cronjob_fails() {
    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption {
        block_time: Duration::from_seconds(1),
        ..TestOption::default()
    });

    let tester_code = ContractBuilder::new(Box::new(failing_tester::instantiate))
        .with_cron_execute(Box::new(failing_tester::cron_execute))
        .build();

    let tester_code_hash = suite
        .upload(&mut accounts.owner, tester_code)
        .await
        .should_succeed()
        .code_hash;

    let cron = suite
        .instantiate(
            &mut accounts.owner,
            tester_code_hash,
            &Empty {},
            "cron1",
            Some("cron1"),
            None,
            Coins::default(),
        )
        .await
        .should_succeed()
        .address;

    let mut new_cfg = suite.query_config().unwrap();
    new_cfg.cronjobs = btree_map! {
        // cron1 has interval of 0, meaning it's to be called every block.
        cron => Duration::from_seconds(0),
    };

    suite
        .configure::<Json>(&mut accounts.owner, Some(new_cfg), None)
        .await
        .should_succeed();

    // Before the block, storage key `b"foo"` should have the value `b"init"`.
    suite
        .query_wasm_raw(cron, *b"foo")
        .should_succeed_and_equal(Some(Binary::from(*b"init")));

    // Advance block and trigger the cronjob
    let res = suite.make_empty_block().await.block_outcome;
    assert_eq!(res.cron_outcomes.len(), 1);

    // The cronjob attempts to overwrite the value with `b"cron_execute"`.
    // But it then fails before returning, so the change it discarded.
    suite
        .query_wasm_raw(cron, *b"foo")
        .should_succeed_and_equal(Some(Binary::from(*b"init")));
}
