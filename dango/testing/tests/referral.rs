use {
    dango_testing::{Factory, Preset, TestAccount, TestOption, TestSuite, setup_test},
    dango_types::{
        account_factory::{ExecuteMsg, QueryAccountRequest, RegisterUserData},
        taxman::{
            self, QueryConfigRequest, QueryReferralStatsRequest, QueryReferrerRequest, ShareRatio,
            UserReferralData,
        },
    },
    grug::{Addr, Addressable, Coins, HashExt, QuerierExt, ResultExt, TxOutcome, Udec128, Uint128},
};

#[test]
fn share_ratio() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    let mut cfg = suite
        .query_wasm_smart(contracts.taxman, QueryConfigRequest {})
        .should_succeed();

    // For testing, the volume requirement is set to 0.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    // Set the required volume to $10k.
    cfg.referral.volume_to_be_referrer = Uint128::new(10_000);

    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure { new_cfg: cfg },
            Coins::new(),
        )
        .should_succeed();

    // Should fail now since User2 has 0 volume.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user2,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_fail_with_error(
        "you must have at least a volume of $10000 to become a referrer, traded volume: $0",
    );
}

#[test]
fn referral_during_user_register() {
    let (mut suite, mut accounts, codes, contracts, ..) = setup_test(TestOption::preset_test());

    suite.make_empty_block();

    // Set share ratio for User1 to 50%.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    let chain_id = suite.chain_id.clone();

    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account_single.to_bytes().hash256(),
        true,
    );

    println!("New user address: {}", user.address());

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
                referrer: Some(1),
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
            .query_wasm_smart(contracts.taxman, QueryReferrerRequest { user: user_index })
            .should_succeed(),
        Some(1)
    );
    // Ensure User1's referee count is now 1.
    assert_eq!(
        query_referrer_stats(&mut suite, contracts.taxman, 1).referee_count,
        1
    );
}

#[test]
fn referral_after_user_register() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    // Set share ratio for User1, 2 and 3.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user2,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user3,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    // Initially, User1 has 0 referees.
    assert_eq!(
        query_referrer_stats(&mut suite, contracts.taxman, 1).referee_count,
        0
    );

    // Make User1 refer User2.
    suite
        .execute(
            &mut accounts.user2,
            contracts.taxman,
            &taxman::ExecuteMsg::SetReferral {
                referrer: 1,
                referee: 2,
            },
            Coins::new(),
        )
        .should_succeed();

    // Ensure User2's referrer is User1.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.taxman, QueryReferrerRequest { user: 2 })
            .should_succeed(),
        Some(1)
    );

    // User1 should now have 1 referee.
    assert_eq!(
        query_referrer_stats(&mut suite, contracts.taxman, 1).referee_count,
        1
    );

    // Try to replace the User2 referrer (should fail).
    suite
        .execute(
            &mut accounts.user2,
            contracts.taxman,
            &taxman::ExecuteMsg::SetReferral {
                referrer: 3,
                referee: 2,
            },
            Coins::new(),
        )
        .should_fail_with_error("user 2 already has a referrer and it can't be changed");

    // Make User1 refer User3.
    suite
        .execute(
            &mut accounts.user3,
            contracts.taxman,
            &taxman::ExecuteMsg::SetReferral {
                referrer: 1,
                referee: 3,
            },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.taxman, QueryReferrerRequest { user: 3 })
            .should_succeed(),
        Some(1)
    );

    // User1 should now have 2 referees.
    assert_eq!(
        query_referrer_stats(&mut suite, contracts.taxman, 1).referee_count,
        2
    );

    // Make User2 refer User4.
    suite
        .execute(
            &mut accounts.user4,
            contracts.taxman,
            &taxman::ExecuteMsg::SetReferral {
                referrer: 2,
                referee: 4,
            },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.taxman, QueryReferrerRequest { user: 4 })
            .should_succeed(),
        Some(2)
    );

    // User2 should now have 1 referee.
    assert_eq!(
        query_referrer_stats(&mut suite, contracts.taxman, 2).referee_count,
        1
    );
}

fn query_referrer_stats(suite: &mut TestSuite, taxman: Addr, user: u32) -> UserReferralData {
    suite
        .query_wasm_smart(taxman, QueryReferralStatsRequest { user })
        .should_succeed()
}

fn set_share_ratio(
    suite: &mut TestSuite,
    taxman: Addr,
    user: &mut TestAccount,
    ratio: Udec128,
) -> TxOutcome {
    suite.execute(
        user,
        taxman,
        &taxman::ExecuteMsg::SetFeeShareRatio(ShareRatio::new(ratio).unwrap()),
        Coins::new(),
    )
}
