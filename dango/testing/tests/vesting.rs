use {
    dango_testing::{setup_test_naive, Accounts, TestSuite},
    dango_types::vesting::{self, QueryPositionRequest, Schedule, VestingStatus},
    grug::{
        Addr, Addressable, Coin, Coins, Duration, Inner, MultiplyFraction, ResultExt, Timestamp,
        Udec128, Uint128,
    },
    grug_app::NaiveProposalPreparer,
    std::sync::LazyLock,
    test_case::test_case,
};

static TEST_AMOUNT: LazyLock<Coin> = LazyLock::new(|| Coin::new("uusdc", 100).unwrap());

const MONTH_IN_SECONDS: Timestamp = Timestamp::from_seconds(60 * 60 * 24 * 30);
const DAY_IN_SECONDS: Timestamp = Timestamp::from_seconds(60 * 60 * 24);

fn setup_test() -> (TestSuite<NaiveProposalPreparer>, Accounts, Addr) {
    let (suite, accounts, _codes, contracts) = setup_test_naive();

    (suite, accounts, contracts.vesting)
}

#[test_case(
    0,
    0,
    0,
    Coins::default(),
    Some("invalid payment: expecting 1 coins, found 0");
    "no funds"
)]
#[test_case(
    99,
    0,
    0,
    TEST_AMOUNT.clone().into(),
    None;
    "ok start time before now"
)]
#[test_case(
    100,
    0,
    0,
    TEST_AMOUNT.clone().into(),
    None;
    "ok no cliff no vesting"
)]
#[test_case(
    100,
    200,
    0,
    TEST_AMOUNT.clone().into(),
    None;
    "ok no vesting"
)]
#[test_case(
    100,
    0,
    300,
    TEST_AMOUNT.clone().into(),
    None;
    "ok no cliff"
)]
#[test_case(
    100,
    200,
    300,
    TEST_AMOUNT.clone().into(),
    None;
    "ok"
)]
fn creation_cases(
    start_time: u128,
    cliff: u128,
    vesting: u128,
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
                start_time: Duration::from_seconds(start_time),
                cliff: Duration::from_seconds(cliff),
                vesting: Duration::from_seconds(vesting),
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
fn before_unlocking_starting_time() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: suite.block.timestamp - MONTH_IN_SECONDS,
                    cliff: MONTH_IN_SECONDS * 9,
                    vesting: MONTH_IN_SECONDS * 27,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go 1 day before cliff ends
    {
        suite.block_time = MONTH_IN_SECONDS * 9 - DAY_IN_SECONDS;

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("nothing to claim");
    }

    // Go at the end of the cliff. Claim should be possible
    {
        suite.block_time = DAY_IN_SECONDS;

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
                        .checked_mul_dec_floor(Udec128::checked_from_ratio(1, 3).unwrap())
                        .unwrap(),
            );
    }

    // Go at 66.66% of the vesting period
    {
        suite.block_time = MONTH_IN_SECONDS * 9;

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
                        .checked_mul_dec_floor(Udec128::checked_from_ratio(2, 3).unwrap())
                        .unwrap(),
            );
    }

    // Go at the end of the vesting period
    {
        suite.block_time = MONTH_IN_SECONDS * 9;

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
fn after_unlocking_starting_time() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: suite.block.timestamp + MONTH_IN_SECONDS,
                    cliff: MONTH_IN_SECONDS * 9,
                    vesting: MONTH_IN_SECONDS * 27,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go 1 day before cliff ends
    {
        suite.block_time = MONTH_IN_SECONDS * 9 - DAY_IN_SECONDS;

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("nothing to claim");
    }

    // Go at the end of the cliff. Claim should not possible (1 month missing)
    {
        suite.block_time = DAY_IN_SECONDS;

        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_fail_with_error("nothing to claim");
    }

    // Go at 1 month after unlocking cliff ends
    // This match with the finish of the vesting cliff
    {
        suite.block_time = MONTH_IN_SECONDS;

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
                        .checked_mul_dec_floor(Udec128::checked_from_ratio(1, 3).unwrap())
                        .unwrap(),
            );
    }

    // Go at 66.66% of the vesting period
    {
        suite.block_time = MONTH_IN_SECONDS * 9;

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
                        .checked_mul_dec_floor(Udec128::checked_from_ratio(2, 3).unwrap())
                        .unwrap(),
            );
    }

    // Go at the end of the vesting period
    {
        suite.block_time = MONTH_IN_SECONDS * 9;

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
fn terminate_before_unlocking_starting_time_never_claimed() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: suite.block.timestamp - MONTH_IN_SECONDS,
                    cliff: MONTH_IN_SECONDS * 9,
                    vesting: MONTH_IN_SECONDS * 27,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    let epoche = epoche(MONTH_IN_SECONDS * 27, TEST_AMOUNT.amount);

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go 1 month after unlocking cliff finish.
    // Terminate user position.
    // In this situation, the vested amount so far of the user is
    // 1 + 9 + 1 / 27 * 100 = 11 / 27 * 100 = 40
    // The unlocked amount is
    // 9 + 1 / 27 * 100 = 10 / 27 * 100 = 37
    {
        suite.block_time = MONTH_IN_SECONDS * 10;

        suite
            .execute(
                &mut accounts.owner,
                vesting_addr,
                &vesting::ExecuteMsg::TerminatePosition { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check the status of the position after terminate
        suite
            .query_wasm_smart(vesting_addr, QueryPositionRequest { idx: 1 })
            .should_succeed_and(|position| {
                position.vesting_status == VestingStatus::Terminated(Uint128::new(40))
                    && position.claimable_amount == Uint128::new(37)
            });

        suite.block_time = Timestamp::default();

        // Claim
        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check the balance of the user
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + Uint128::new(37));

        // Go forward 3 epoche to claim all tokens
        suite.block_time = epoche * 3;

        // Claim
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

        // Check the balance of the user
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + Uint128::new(40));
    }
}

#[test]
fn terminate_before_unlocking_starting_time_with_claimed() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: suite.block.timestamp - MONTH_IN_SECONDS,
                    cliff: MONTH_IN_SECONDS * 9,
                    vesting: MONTH_IN_SECONDS * 27,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    let epoche = epoche(MONTH_IN_SECONDS * 27, TEST_AMOUNT.amount);

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go 1 month after unlocking cliff finish.
    // The user claim the tokens
    // In this situation, the vested amount so far of the user is
    // 1 + 9 + 1 / 27 * 100 = 11 / 27 * 100 = 40
    // The unlocked amount is
    // 9 + 1 / 27 * 100 = 10 / 27 * 100 = 37
    {
        suite.block_time = MONTH_IN_SECONDS * 10;

        // Claim
        suite
            .execute(
                &mut accounts.relayer,
                vesting_addr,
                &vesting::ExecuteMsg::Claim { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check the balance of the user
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + Uint128::new(37));
    }

    // Go 1 month after
    // Terminate user position.
    // In this situation, the vested amount so far of the user is
    // 1 + 9 + 2 / 27 * 100 = 12 / 27 * 100 = 44
    // The unlocked amount is
    // 9 + 2 / 27 * 100 = 11 / 27 * 100 = 40
    {
        suite.block_time = MONTH_IN_SECONDS;

        suite
            .execute(
                &mut accounts.owner,
                vesting_addr,
                &vesting::ExecuteMsg::TerminatePosition { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check the status of the position after terminate
        suite
            .query_wasm_smart(vesting_addr, QueryPositionRequest { idx: 1 })
            .should_succeed_and(|position| {
                position.vesting_status == VestingStatus::Terminated(Uint128::new(44))
                    && position.claimable_amount == Uint128::new(3)
                    && position.claimed_amount == Uint128::new(37)
            });

        // 4 epoche is needed to claim all tokens
        // Instead wait for 8 epoche to check if there are any problems waiting more than needed
        suite.block_time = epoche * 8;

        // Claim
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

        // Check the balance of the user
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + Uint128::new(44));
    }
}

#[test]
fn terminate_after_unlocking_starting_time() {
    let (mut suite, mut accounts, vesting_addr) = setup_test();

    suite
        .execute(
            &mut accounts.owner,
            vesting_addr,
            &vesting::ExecuteMsg::CreatePosition {
                user: accounts.relayer.address(),
                schedule: Schedule {
                    start_time: suite.block.timestamp + MONTH_IN_SECONDS,
                    cliff: MONTH_IN_SECONDS * 9,
                    vesting: MONTH_IN_SECONDS * 27,
                },
            },
            TEST_AMOUNT.clone(),
        )
        .should_succeed();

    let initial_balance = suite
        .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
        .should_succeed();

    // Go 2 month after unlocking cliff finish.
    // Terminate user position.
    // In this situation, the vested amount so far of the user is
    // -1 + 9 + 2 / 27 * 100 = 9 / 27 * 100 = 37
    // The unlocked amount is
    // 9 + 2 / 27 * 100 = 11 / 27 * 100 = 40
    {
        suite.block_time = MONTH_IN_SECONDS * 11;

        suite
            .execute(
                &mut accounts.owner,
                vesting_addr,
                &vesting::ExecuteMsg::TerminatePosition { idx: 1 },
                Coins::default(),
            )
            .should_succeed();

        // Check the status of the position after terminate
        suite
            .query_wasm_smart(vesting_addr, QueryPositionRequest { idx: 1 })
            .should_succeed_and(|position| {
                position.vesting_status == VestingStatus::Terminated(Uint128::new(37))
                    && position.claimable_amount == Uint128::new(37)
            });

        suite.block_time = Timestamp::default();

        // Claim
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

        // Check the balance of the user
        suite
            .query_balance(&accounts.relayer, TEST_AMOUNT.denom.clone())
            .should_succeed_and_equal(initial_balance + Uint128::new(37));
    }
}

// Epoche for unlock 1 token
fn epoche(total_duration: Timestamp, vesting_amount: Uint128) -> Timestamp {
    Timestamp::from_nanos(total_duration.into_nanos() / vesting_amount.into_inner())
}
