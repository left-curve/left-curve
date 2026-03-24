use {
    dango_testing::{Factory, Preset, TestAccount, TestOption, setup_test_naive},
    dango_types::{
        Dimensionless, UsdValue,
        account_factory::{self, RegisterUserData},
        perps::{self, FeeShareRatio},
    },
    grug::{Addr, Addressable, Coins, HashExt, QuerierExt, ResultExt, Signer, TxOutcome},
    grug_app::NaiveProposalPreparer,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Register a referral relationship during user registration via the account
/// factory (the `referrer` field on `RegisterUser`).
#[test]
fn referral_during_user_register() {
    let (mut suite, mut accounts, codes, contracts, ..) =
        setup_test_naive(TestOption::preset_test());

    suite.make_empty_block();

    // User1 (index 1) sets a fee share ratio so they can be a referrer.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    let chain_id = suite.chain_id.clone();

    // Create a new user and predict its address.
    let user = TestAccount::new_random().predict_address(
        contracts.account_factory,
        3,
        codes.account.to_bytes().hash256(),
        true,
    );

    // Register the new user with User1 as referrer.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
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

    // Look up the new user's index.
    let user_index = suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountRequest {
                address: user.address(),
            },
        )
        .should_succeed()
        .owner;

    // The new user's referrer should be User1 (index 1).
    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryReferrerRequest {
                referee: user_index,
            },)
            .should_succeed(),
        Some(1),
    );
}

/// Set a referral relationship after the user has already registered, and
/// verify immutability + multiple referees.
#[test]
fn referral_after_user_register() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    // User1, User2, and User3 all set fee share ratios.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user2,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user3,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    // User2 sets User1 as referrer.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 1,
                referee: 2,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify User2's referrer is User1.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryReferrerRequest { referee: 2 },)
            .should_succeed(),
        Some(1),
    );

    // Trying to change User2's referrer should fail (immutable).
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 3,
                referee: 2,
            }),
            Coins::new(),
        )
        .should_fail_with_error("referee 2 already has a referrer");

    // User3 also sets User1 as referrer.
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 1,
                referee: 3,
            }),
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryReferrerRequest { referee: 3 },)
            .should_succeed(),
        Some(1),
    );

    // User4 sets User2 as referrer (chain: User1 <- User2 <- User4).
    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 2,
                referee: 4,
            }),
            Coins::new(),
        )
        .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryReferrerRequest { referee: 4 },)
            .should_succeed(),
        Some(2),
    );
}

/// A user cannot refer themselves.
#[test]
fn referral_self_refer_fails() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 1,
                referee: 1,
            }),
            Coins::new(),
        )
        .should_fail_with_error("a user cannot refer themselves");
}

/// A referral cannot be set if the referrer has no fee share ratio.
#[test]
fn referral_without_share_ratio_fails() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    // User1 has NOT set a share ratio. Trying to set User1 as referrer should fail.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 1,
                referee: 2,
            }),
            Coins::new(),
        )
        .should_fail_with_error("referrer 1 has no fee share ratio set");
}

/// Only the referee (or the account factory) can set the referral relationship.
#[test]
fn referral_wrong_caller_fails() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    // User3 tries to set the referral for User2 — should fail.
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetReferral {
                referrer: 1,
                referee: 2,
            }),
            Coins::new(),
        )
        .should_fail_with_error("caller is not the account factory or the referee");
}

/// The fee share ratio can only increase, never decrease.
#[test]
fn modify_share_ratio() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    // Set initial share ratio to 20%.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(20),
    )
    .should_succeed();

    // Verify the stored ratio.
    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryFeeShareRatioRequest {
                referrer: 1
            },)
            .should_succeed(),
        Some(Dimensionless::new_percent(20)),
    );

    // Try to lower the share ratio — should fail.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(10),
    )
    .should_fail_with_error("fee share ratio can only increase");

    // Increase the share ratio — should succeed.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(50),
    )
    .should_succeed();

    assert_eq!(
        suite
            .query_wasm_smart(contracts.perps, perps::QueryFeeShareRatioRequest {
                referrer: 1
            },)
            .should_succeed(),
        Some(Dimensionless::new_percent(50)),
    );
}

/// Setting the fee share ratio requires sufficient perps trading volume
/// when `volume_to_be_referrer` is non-zero.
#[test]
fn set_share_ratio_requires_volume() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    // Configure the perps contract to require $10,000 volume to become a
    // referrer.
    let mut param: perps::Param = suite
        .query_wasm_smart(contracts.perps, perps::QueryParamRequest {})
        .should_succeed();

    param.referral.volume_to_be_referrer = UsdValue::new_int(10_000);

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param,
                pair_params: Default::default(),
            }),
            Coins::new(),
        )
        .should_succeed();

    // User2 has zero volume — should fail.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user2,
        Dimensionless::new_percent(50),
    )
    .should_fail_with_error("insufficient perps volume to become a referrer");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn set_fee_share_ratio(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: &mut dyn Signer,
    ratio: FeeShareRatio,
) -> TxOutcome {
    suite.execute(
        user,
        perps,
        &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetFeeShareRatio { share_ratio: ratio }),
        Coins::new(),
    )
}
