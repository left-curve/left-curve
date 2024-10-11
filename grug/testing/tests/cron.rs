use {
    grug_testing::TestBuilder,
    grug_types::{btree_map, Coin, Coins, ConfigUpdates, Duration, ResultExt, Salt, Timestamp},
    grug_vm_rust::ContractBuilder,
    std::{collections::BTreeMap, str::FromStr},
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

struct Balances {
    uatom: u128,
    uosmo: u128,
    umars: u128,
}

#[test]
fn cronjob_works() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("larry", [("uatom", 100), ("uosmo", 100), ("umars", 100)])
        .unwrap()
        .add_account("jake", Coins::new())
        .unwrap()
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("larry")
        .unwrap()
        .build()
        .unwrap();

    let tester_code = ContractBuilder::new(Box::new(tester::instantiate))
        .with_cron_execute(Box::new(tester::cron_execute))
        .build();

    let receiver = accounts["jake"].address;

    // Block time: 1
    //
    // Upload the tester contract code.
    let tester_code_hash = suite
        .upload(accounts.get_mut("larry").unwrap(), tester_code)
        .unwrap();

    // Block time: 2
    //
    // Deploy three tester contracts with different jobs.
    // Each contract is given an initial coin balance.
    let cron1 = suite
        .instantiate(
            accounts.get_mut("larry").unwrap(),
            tester_code_hash,
            Salt::from_str("cron1").unwrap(),
            &tester::Job {
                receiver,
                coin: Coin::new("uatom", 1).unwrap(),
            },
            Coins::one("uatom", 3).unwrap(),
        )
        .unwrap();

    // Block time: 3
    let cron2 = suite
        .instantiate(
            accounts.get_mut("larry").unwrap(),
            tester_code_hash,
            Salt::from_str("cron2").unwrap(),
            &tester::Job {
                receiver,
                coin: Coin::new("uosmo", 1).unwrap(),
            },
            Coins::one("uosmo", 3).unwrap(),
        )
        .unwrap();

    // Block time: 4
    let cron3 = suite
        .instantiate(
            accounts.get_mut("larry").unwrap(),
            tester_code_hash,
            Salt::from_str("cron3").unwrap(),
            &tester::Job {
                receiver,
                coin: Coin::new("umars", 1).unwrap(),
            },
            Coins::one("umars", 3).unwrap(),
        )
        .unwrap();

    // Block time: 5
    //
    // Update the config to add the cronjobs.
    let updates = ConfigUpdates {
        cronjobs: Some(btree_map! {
            // cron1 has interval of 0, meaning it's to be called every block.
            cron1 => Duration::from_seconds(0),
            cron2 => Duration::from_seconds(2),
            cron3 => Duration::from_seconds(3),
        }),
        ..Default::default()
    };

    // cron1 scheduled at 5
    // cron2 scheduled at 7
    // cron3 scheduled at 8
    suite
        .configure(accounts.get_mut("larry").unwrap(), updates, BTreeMap::new())
        .unwrap();

    // Make some blocks.
    // After each block, check that Jake has the correct balances.
    for balances in [
        // Block time: 6
        //
        // cron1 sends 1 uatom, rescheduled to 6
        Balances {
            uatom: 1,
            uosmo: 0,
            umars: 0,
        },
        // Block time: 7
        //
        // cron1 sends 1 uatom, rescheduled to 7
        // cron2 sends 1 uosmo, rescheduled to 9
        Balances {
            uatom: 2,
            uosmo: 1,
            umars: 0,
        },
        // Block time: 8
        //
        // cron1 sends 1 uatom, rescheduled to 8 (it runs out of coins here)
        // cron3 sends 1 umars, rescheduled to 11
        Balances {
            uatom: 3,
            uosmo: 1,
            umars: 1,
        },
        // Block time: 9
        //
        // cron1 errors because it's out of coins
        // cron2 sends 1 uosmo, rescheduled to 11
        Balances {
            uatom: 3,
            uosmo: 2,
            umars: 1,
        },
        // Block time: 10
        //
        // Nothing happens
        Balances {
            uatom: 3,
            uosmo: 2,
            umars: 1,
        },
        // Block time: 11
        //
        // cron2 sends 1 uosmo (runs out of coins), rescheduled to 13
        // cron3 sends 1 umars, rescheduled to 14
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 12
        //
        // Nothing happens
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 13
        //
        // cron2 errors
        // Otherwise nothing happens
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 2,
        },
        // Block time: 14
        //
        // cron3 sends 1 umars, runs out of coins
        Balances {
            uatom: 3,
            uosmo: 3,
            umars: 3,
        },
    ] {
        // The balances Jake is expected to have at time point
        let mut expect = Coins::new();
        expect
            .insert(Coin::new("uatom", balances.uatom).unwrap())
            .unwrap();
        expect
            .insert(Coin::new("uosmo", balances.uosmo).unwrap())
            .unwrap();
        expect
            .insert(Coin::new("umars", balances.umars).unwrap())
            .unwrap();

        // Advance block
        suite.make_empty_block().unwrap();

        // Check the balances are correct
        let actual = suite.query_balances(&accounts["jake"]).should_succeed();
        assert_eq!(actual, expect);
    }
}
