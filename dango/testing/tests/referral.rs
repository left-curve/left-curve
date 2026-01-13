use {
    dango_testing::{Preset, TestOption, TestSuite, setup_test},
    dango_types::referral::{ExecuteMsg, QueryRefereeCountRequest, QueryReferrerRequest},
    grug::{Addr, Coins, QuerierExt, ResultExt},
};

#[test]
fn test() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    // Initially, User1 has 0 referees.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.referral.clone(), 1),
        0
    );

    // Make User1 refer User2.
    suite
        .execute(
            &mut accounts.user2,
            contracts.referral,
            &ExecuteMsg::Referral { referrer_index: 1 },
            Coins::new(),
        )
        .should_succeed();

    // Ensure User2's referrer is User1.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.referral.clone(), QueryReferrerRequest {
                referee_index: 2
            })
            .should_succeed(),
        1
    );

    // User1 should now have 1 referee.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.referral.clone(), 1),
        1
    );

    // Try to replace the User2 referrer (should fail).
    suite
        .execute(
            &mut accounts.user2,
            contracts.referral,
            &ExecuteMsg::Referral { referrer_index: 3 },
            Coins::new(),
        )
        .should_fail_with_error("referral already registered for this user");

    // Make User1 refer User3.
    suite
        .execute(
            &mut accounts.user3,
            contracts.referral.clone(),
            &ExecuteMsg::Referral { referrer_index: 1 },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.referral.clone(), QueryReferrerRequest {
                referee_index: 3
            })
            .should_succeed(),
        1
    );

    // User1 should now have 2 referees.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.referral.clone(), 1),
        2
    );

    // Make User2 refer User4.
    suite
        .execute(
            &mut accounts.user4,
            contracts.referral.clone(),
            &ExecuteMsg::Referral { referrer_index: 2 },
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.referral.clone(), QueryReferrerRequest {
                referee_index: 4
            })
            .should_succeed(),
        2
    );

    // User2 should now have 1 referee.
    assert_eq!(
        query_referrer_count(&mut suite, contracts.referral.clone(), 2),
        1
    );
}

fn query_referrer_count(suite: &mut TestSuite, referral_contract: Addr, user: u32) -> u32 {
    suite
        .query_wasm_smart(referral_contract, QueryRefereeCountRequest { user })
        .should_succeed()
}
