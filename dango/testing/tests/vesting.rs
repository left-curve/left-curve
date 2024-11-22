use {
    dango_testing::{setup_test_naive, Accounts, TestSuite},
    dango_types::vesting::{self, Schedule},
    grug::{
        Addr, Addressable, Coin, Coins, Duration, MultiplyFraction, NumberConst, ResultExt,
        Udec128, Uint128,
    },
    grug_app::NaiveProposalPreparer,
    std::sync::LazyLock,
    test_case::test_case,
};

static TEST_AMOUNT: LazyLock<Coin> = LazyLock::new(|| Coin::new("uusdc", 100).unwrap());

fn setup_test() -> (TestSuite<NaiveProposalPreparer>, Accounts, Addr) {
    let (mut suite, accounts, _codes, contracts) = setup_test_naive();

    suite.block_time = Duration::from_seconds(0);
    suite.block.timestamp = Duration::from_seconds(100);

    (suite, accounts, contracts.vesting)
}

#[test_case(
    None,
    None,
    None,
    Coins::default(),
    Some("invalid payment: expecting 1 coins, found 0");
    "no funds"
)]
#[test_case(
    Some(99),
    None,
    None,
    TEST_AMOUNT.clone().into(),
    Some("invalid start time");
    "invalid start time"
)]
#[test_case(
    Some(100),
    None,
    None,
    TEST_AMOUNT.clone().into(),
    None;
    "ok no cliff no vesting"
)]
#[test_case(
    Some(100),
    Some(200),
    None,
    TEST_AMOUNT.clone().into(),
    None;
    "ok no vesting"
)]
#[test_case(
    Some(100),
    None,
    Some(300),
    TEST_AMOUNT.clone().into(),
    None;
    "ok no cliff"
)]
#[test_case(
    Some(100),
    Some(200),
    Some(300),
    TEST_AMOUNT.clone().into(),
    None;
    "ok"
)]
fn creation_cases(
    start_time: Option<u128>,
    cliff: Option<u128>,
    vesting: Option<u128>,
    coins: Coins,
    maybe_err: Option<&str>,
) {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    let res = suite.execute(
        &mut accounts.owner,
        vesting_addr,
        &vesting::ExecuteMsg::CreatePosition {
            user: accounts.relayer.address(),
            schedule: Schedule {
                start_time: start_time.map(Duration::from_seconds),
                cliff: cliff.map(Duration::from_seconds),
                vesting: vesting.map(Duration::from_seconds),
            },
        },
        coins,
    );

    if let Some(err) = maybe_err {
        res.should_fail_with_error(err);
    } else {
        res.should_succeed();
    }
}

#[test]
fn cliff() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: None,
                    cliff: Some(Duration::from_seconds(100)),
                    vesting: None,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    // Go 50 sec forward (before cliff ends). Claim should not be possible
    {
        suite.block_time = Duration::from_seconds(50);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("nothing to claim during cliff phase");

        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_succeed_and(|res| res.claimable_amount == Uint128::ZERO);
    }

    // Go 50 sec forward (after cliff ends). Now should be claimable
    {
        // Create an empty block to check the query (sanity check)
        suite.make_empty_block();

        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_succeed_and(|res| res.claimable_amount == res.amount.amount);

        let balance_before = suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed();

        suite.block_time = Duration::from_seconds(0);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check if the position is removed
        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_fail_with_error("not found");

        // Check if the balance is correct
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(balance_before + TEST_AMOUNT.amount);
    }
}

#[test]
fn vesting() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: Some(Duration::from_seconds(110)),
                    cliff: None,
                    vesting: Some(Duration::from_seconds(100)),
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    // Go before start_time.
    // claim should not be possible
    {
        suite.block_time = Duration::from_seconds(9);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("vesting has not started yet");
    }

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go at 10% of the vesting period and claim
    {
        suite.block_time = Duration::from_seconds(11);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(
                initial_balance
                    + TEST_AMOUNT
                        .amount
                        .checked_mul_dec_floor(Udec128::new_percent(10))
                        .unwrap(),
            );
    }

    // Go at 70% of the vesting period and claim
    {
        suite.block_time = Duration::from_seconds(60);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(
                initial_balance
                    + TEST_AMOUNT
                        .amount
                        .checked_mul_dec_floor(Udec128::new_percent(70))
                        .unwrap(),
            );
    }

    // Go at 120% of the vesting period and claim
    {
        suite.block_time = Duration::from_seconds(50);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + TEST_AMOUNT.amount);

        // Check if the position is removed
        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_fail_with_error("not found");
    }
}

#[test]
fn cliff_and_vesting() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: None,
                    cliff: Some(Duration::from_seconds(50)),
                    vesting: Some(Duration::from_seconds(100)),
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    // Go 25 sec forward (before cliff ends). Claim should not be possible
    {
        suite.block_time = Duration::from_seconds(25);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("nothing to claim during cliff phase");

        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_succeed_and(|res| res.claimable_amount == Uint128::ZERO);
    }

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go after cliff at 25% of vesting
    {
        suite.block_time = Duration::from_seconds(25 + 25);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(
                initial_balance
                    + TEST_AMOUNT
                        .amount
                        .checked_mul_dec_floor(Udec128::new_percent(25))
                        .unwrap(),
            );
    }

    // Go at 99% of vesting
    {
        suite.block_time = Duration::from_seconds(99 - 25);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(
                initial_balance
                    + TEST_AMOUNT
                        .amount
                        .checked_mul_dec_floor(Udec128::new_percent(99))
                        .unwrap(),
            );
    }

    // Go at 100% of vesting
    {
        suite.block_time = Duration::from_seconds(1);

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + TEST_AMOUNT.amount);

        // Check if the position is removed
        suite
            .query_wasm_smart(vesting_addr, vesting::QueryPositionRequest { idx: 1 })
            .should_fail_with_error("not found");
    }
}
