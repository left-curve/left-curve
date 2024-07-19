use {
    anyhow::ensure,
    grug_testing::TestBuilder,
    grug_types::{Coin, Coins, Duration, NonZero, Timestamp, Uint128},
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
        JOB.save(ctx.storage, &job)?;

        Ok(Response::new())
    }

    pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
        let job = JOB.load(ctx.storage)?;

        Ok(Response::new().add_message(Message::transfer(job.receiver, job.coin)?))
    }
}

struct Balances {
    uatom: u128,
    uosmo: u128,
    umars: u128,
}

#[test]
fn cronjob_works() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account(
            "larry",
            Coins::try_from([
                Coin::new("uatom", NonZero::new(Uint128::new(100))),
                Coin::new("uosmo", NonZero::new(Uint128::new(100))),
                Coin::new("umars", NonZero::new(Uint128::new(100))),
            ])?,
        )?
        .add_account("jake", Coins::new_empty())?
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("larry")?
        .build()?;

    let tester_code = ContractBuilder::new(Box::new(tester::instantiate))
        .with_cron_execute(Box::new(tester::cron_execute))
        .build();

    // Block time: 1
    //
    // Upload the tester contract code.
    let tester_code_hash = suite.upload(&accounts["larry"], tester_code)?;

    // Block time: 2
    //
    // Deploy three tester contracts with different jobs.
    // Each contract is given an initial coin balance.
    let cron1 = suite.instantiate(
        &accounts["larry"],
        tester_code_hash.clone(),
        "cron1",
        &tester::Job {
            receiver: accounts["jake"].address.clone(),
            coin: Coin::new("uatom", NonZero::new(Uint128::new(1))),
        },
        Coin::new("uatom", NonZero::new(Uint128::new(3))),
    )?;

    // Block time: 3
    let cron2 = suite.instantiate(
        &accounts["larry"],
        tester_code_hash.clone(),
        "cron2",
        &tester::Job {
            receiver: accounts["jake"].address.clone(),
            coin: Coin::new("uosmo", NonZero::new(Uint128::new(1))),
        },
        Coin::new("uosmo", NonZero::new(Uint128::new(3))),
    )?;

    // Block time: 4
    let cron3 = suite.instantiate(
        &accounts["larry"],
        tester_code_hash.clone(),
        "cron3",
        &tester::Job {
            receiver: accounts["jake"].address.clone(),
            coin: Coin::new("umars", NonZero::new(Uint128::new(1))),
        },
        Coin::new("umars", NonZero::new(Uint128::new(3))),
    )?;

    // Block time: 5
    //
    // Update the config to add the cronjobs.
    let mut cfg = suite.query_info().should_succeed()?.config;
    // cron1 has an interval of 0, which means it's to be called every block.
    cfg.cronjobs.insert(cron1, Duration::from_seconds(0));
    cfg.cronjobs.insert(cron2, Duration::from_seconds(2));
    cfg.cronjobs.insert(cron3, Duration::from_seconds(3));

    // cron1 scheduled at 5
    // cron2 scheduled at 7
    // cron3 scheduled at 8
    suite.configure(&accounts["larry"], cfg)?;

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
        let mut expect = Coins::new_empty();
        expect.increase_amount("uatom", balances.uatom.into())?;
        expect.increase_amount("uosmo", balances.uosmo.into())?;
        expect.increase_amount("umars", balances.umars.into())?;

        // Advance block
        suite.make_empty_block()?;

        // Check the balances are correct
        let actual = suite.query_balances(&accounts["jake"]).should_succeed()?;
        ensure!(actual == expect);
    }

    Ok(())
}
