use {
    dango_testing::{Factory, Preset, TestAccount, TestOption, TestSuite, setup_test},
    dango_types::account_factory::{
        ExecuteMsg, QueryAccountRequest, QueryRefereeCountRequest, QueryReferrerRequest,
        RegisterUserData,
    },
    grug::{Addr, Addressable, Coins, HashExt, QuerierExt, ResultExt},
};

#[test]
fn referral_during_user_register() {
    let (mut suite, _, codes, contracts, ..) = setup_test(TestOption::preset_test());

    suite.make_empty_block();

    let chain_id = suite.chain_id.clone();

    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account_single.to_bytes().hash256(),
        true,
    );

    // Register a new user with User1 as referrer.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &ExecuteMsg::RegisterUser {
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 3,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
                referrer_index: Some(1),
            },
            Coins::new(),
        )
        .should_succeed();

    let user_index = suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: user.address(),
        })
        .should_succeed()
        .params
        .into_single()
        .owner;

    // Ensure the new user's referrer is User1.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.account_factory, QueryReferrerRequest {
                user: user_index
            })
            .should_succeed(),
        Some(1)
    );
    // Ensure User1's referee count is now 1.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.account_factory, 1),
        1
    );
}

#[test]
fn referral_after_user_register() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    // Initially, User1 has 0 referees.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.account_factory, 1),
        0
    );

    // Make User1 refer User2.
    suite
        .execute(
            &mut accounts.user2,
            contracts.account_factory,
            &ExecuteMsg::Referral { referrer_index: 1 },
            Coins::new(),
        )
        .should_succeed();

    // Ensure User2's referrer is User1.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.account_factory, QueryReferrerRequest {
                user: 2
            })
            .should_succeed(),
        Some(1)
    );

    // User1 should now have 1 referee.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.account_factory, 1),
        1
    );

    // Try to replace the User2 referrer (should fail).
    suite
        .execute(
            &mut accounts.user2,
            contracts.account_factory,
            &ExecuteMsg::Referral { referrer_index: 3 },
            Coins::new(),
        )
        .should_fail_with_error("referral already registered for this user");

    // Make User1 refer User3.
    suite
        .execute(
            &mut accounts.user3,
            contracts.account_factory,
            &ExecuteMsg::Referral { referrer_index: 1 },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.account_factory, QueryReferrerRequest {
                user: 3
            })
            .should_succeed(),
        Some(1)
    );

    // User1 should now have 2 referees.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.account_factory, 1),
        2
    );

    // Make User2 refer User4.
    suite
        .execute(
            &mut accounts.user4,
            contracts.account_factory,
            &ExecuteMsg::Referral { referrer_index: 2 },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.account_factory, QueryReferrerRequest {
                user: 4
            })
            .should_succeed(),
        Some(2)
    );

    // User2 should now have 1 referee.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.account_factory, 2),
        1
    );
}

fn query_referrer_count(suite: &mut TestSuite, account_factory: Addr, user: u32) -> u32 {
    suite
        .query_wasm_smart(account_factory, QueryRefereeCountRequest { user })
        .should_succeed()
}
