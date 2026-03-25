use {
    dango_testing::{Factory, Preset, TestAccount, TestOption, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        account_factory::{self, RegisterUserData},
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{
            self, CommissionReboundRate, FeeShareRatio, Referee, ReferrerSettings,
            ReferrerStatsOrderBy, ReferrerStatsOrderIndex, UserReferralData,
        },
    },
    grug::{
        Addr, Addressable, Coins, HashExt, NumberConst, Order as IterationOrder, QuerierExt,
        ResultExt, Signer, Timestamp, TxOutcome, Udec128, Uint128, btree_map,
    },
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
        query_referral_settings(&suite, contracts.perps, 1)
            .unwrap()
            .share_ratio,
        Dimensionless::new_percent(20),
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
        query_referral_settings(&suite, contracts.perps, 1)
            .unwrap()
            .share_ratio,
        Dimensionless::new_percent(50),
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

/// Commission rebound override: only owner can set, overrides volume tiers,
/// removing the override falls back to volume-based calculation.
#[test]
fn commission_rebound_override() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    // Configure commission rebound tiers.
    let mut param = suite
        .query_wasm_smart(contracts.perps, perps::QueryParamRequest {})
        .should_succeed();

    param.referral.commission_rebound_default = CommissionReboundRate::new_percent(10);
    param.referral.commission_rebound_by_volume = btree_map! {
        UsdValue::new_int(100) => CommissionReboundRate::new_percent(20),
    };

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

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user1, 100_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user2, 100_000);

    // User1 becomes a referrer.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(20),
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

    // Default commission rebound is 10%.
    let settings = query_referral_settings(&suite, contracts.perps, 1).unwrap();
    assert_eq!(
        settings.commission_rebound,
        CommissionReboundRate::new_percent(10)
    );

    // Non-owner cannot set override.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetCommissionReboundOverride {
                user: 1,
                commission_rebound: Some(CommissionReboundRate::new_percent(50)),
            }),
            Coins::new(),
        )
        .should_fail_with_error("you don't have the right");

    // Owner sets override to 50%.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetCommissionReboundOverride {
                user: 1,
                commission_rebound: Some(CommissionReboundRate::new_percent(50)),
            }),
            Coins::new(),
        )
        .should_succeed();

    let settings = query_referral_settings(&suite, contracts.perps, 1).unwrap();
    assert_eq!(
        settings.commission_rebound,
        CommissionReboundRate::new_percent(50)
    );

    // Trade to generate volume past the 100 USD tier.
    create_perps_fill(&mut suite, &mut accounts, contracts.perps, 2_000, 1);

    // Override still applies (ignores volume tier).
    let settings = query_referral_settings(&suite, contracts.perps, 1).unwrap();
    assert_eq!(
        settings.commission_rebound,
        CommissionReboundRate::new_percent(50)
    );

    // Owner removes override.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Referral(perps::ReferralMsg::SetCommissionReboundOverride {
                user: 1,
                commission_rebound: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Falls back to volume-based tier (>= 100 → 20%).
    let settings = query_referral_settings(&suite, contracts.perps, 1).unwrap();
    assert_eq!(
        settings.commission_rebound,
        CommissionReboundRate::new_percent(20)
    );
}

/// Query per-referee statistics sorted by volume.
#[test]
fn referrer_stats() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user1, 100_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user2, 100_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user3, 100_000);

    // User1 becomes a referrer.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(20),
    )
    .should_succeed();

    // User2 and User3 set User1 as referrer.
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

    // User2 trades (volume = 1 * 2000 = $2,000).
    create_perps_fill(&mut suite, &mut accounts, contracts.perps, 2_000, 1);

    // User3 trades more (volume = 2 * 2000 = $4,000).
    // Need user3 as taker — swap user1 as maker, user3 as taker.
    place_ask_order(&mut suite, contracts.perps, &mut accounts.user1, 2_000, 2);
    place_market_buy(&mut suite, contracts.perps, &mut accounts.user3, 2);

    // Query stats sorted by volume descending.
    let stats: Vec<(Referee, perps::RefereeStats)> = suite
        .query_wasm_smart(contracts.perps, perps::QueryReferrerToRefereeStatsRequest {
            referrer: 1,
            order_by: ReferrerStatsOrderBy {
                order: IterationOrder::Descending,
                limit: None,
                index: ReferrerStatsOrderIndex::Volume { start_after: None },
            },
        })
        .should_succeed();

    assert_eq!(stats.len(), 2);
    // User3 has more volume, should be first in descending order.
    assert_eq!(stats[0].0, 3);
    assert_eq!(stats[1].0, 2);

    // Query ascending.
    let stats: Vec<(u32, perps::RefereeStats)> = suite
        .query_wasm_smart(contracts.perps, perps::QueryReferrerToRefereeStatsRequest {
            referrer: 1,
            order_by: ReferrerStatsOrderBy {
                order: IterationOrder::Ascending,
                limit: None,
                index: ReferrerStatsOrderIndex::Volume { start_after: None },
            },
        })
        .should_succeed();

    assert_eq!(stats[0].0, 2);
    assert_eq!(stats[1].0, 3);

    let user2_volume = stats[0].1.volume;
    let user3_volume = stats[1].1.volume;

    // start_after: skip user3's volume in descending order → only user2 remains.
    let stats: Vec<(Referee, perps::RefereeStats)> = suite
        .query_wasm_smart(contracts.perps, perps::QueryReferrerToRefereeStatsRequest {
            referrer: 1,
            order_by: ReferrerStatsOrderBy {
                order: IterationOrder::Descending,
                limit: None,
                index: ReferrerStatsOrderIndex::Volume {
                    start_after: Some(user3_volume),
                },
            },
        })
        .should_succeed();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].0, 2);
    assert_eq!(stats[0].1.volume, user2_volume);

    // limit: only return 1 result in descending order → user3 (highest volume).
    let stats: Vec<(Referee, perps::RefereeStats)> = suite
        .query_wasm_smart(contracts.perps, perps::QueryReferrerToRefereeStatsRequest {
            referrer: 1,
            order_by: ReferrerStatsOrderBy {
                order: IterationOrder::Descending,
                limit: Some(1),
                index: ReferrerStatsOrderIndex::Volume { start_after: None },
            },
        })
        .should_succeed();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].0, 3);
}

/// Active users count increments once per referee per day.
#[test]
fn active_referral() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_naive(TestOption::preset_test());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user1, 100_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user2, 100_000);
    deposit_margin(&mut suite, contracts.perps, &mut accounts.user3, 100_000);

    // User1 becomes a referrer.
    set_fee_share_ratio(
        &mut suite,
        contracts.perps,
        &mut accounts.user1,
        Dimensionless::new_percent(20),
    )
    .should_succeed();

    // User2 and User3 set User1 as referrer.
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

    // User2 trades — active_users should be 1.
    create_perps_fill(&mut suite, &mut accounts, contracts.perps, 2_000, 1);

    let data = query_referral_data(&suite, contracts.perps, 1, None);
    assert_eq!(data.active_users, Uint128::new(1));

    // User2 trades again same day — active_users still 1.
    create_perps_fill(&mut suite, &mut accounts, contracts.perps, 2_000, 1);

    let data = query_referral_data(&suite, contracts.perps, 1, None);
    assert_eq!(data.active_users, Uint128::new(1));

    // User3 trades — active_users should be 2.
    place_ask_order(&mut suite, contracts.perps, &mut accounts.user1, 2_000, 1);
    place_market_buy(&mut suite, contracts.perps, &mut accounts.user3, 1);

    let data = query_referral_data(&suite, contracts.perps, 1, None);
    assert_eq!(data.active_users, Uint128::new(2));

    // Next day, User2 trades — active_users should be 3 (cumulative).
    suite.block_time = grug::Duration::from_days(1);
    suite.make_empty_block();

    create_perps_fill(&mut suite, &mut accounts, contracts.perps, 2_000, 1);

    let data = query_referral_data(&suite, contracts.perps, 1, None);
    assert_eq!(data.active_users, Uint128::new(3));
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

fn register_oracle_prices(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &dango_genesis::Contracts,
    eth_price: u128,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
                dango_testing::perps::pair_id() => PriceSource::Fixed {
                    humanized_price: Udec128::new(eth_price),
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
}

fn deposit_margin(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: &mut dyn Signer,
    usd_amount: u128,
) {
    let amount = Uint128::new(usd_amount * 1_000_000);
    suite
        .execute(
            user,
            perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), amount).unwrap(),
        )
        .should_succeed();
}

fn place_ask_order(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: &mut dyn Signer,
    price: u128,
    size: u128,
) {
    suite
        .execute(
            user,
            perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: dango_testing::perps::pair_id(),
                size: Quantity::new_int(-(size as i128)),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(price as i128),
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();
}

fn place_market_buy(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: &mut dyn Signer,
    size: u128,
) {
    suite
        .execute(
            user,
            perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: dango_testing::perps::pair_id(),
                size: Quantity::new_int(size as i128),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();
}

/// Place a limit ask (user1) then a market buy (user2) to produce a fill.
fn create_perps_fill(
    suite: &mut dango_testing::TestSuite<NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    perps: Addr,
    price: u128,
    size: u128,
) {
    place_ask_order(suite, perps, &mut accounts.user1, price, size);
    place_market_buy(suite, perps, &mut accounts.user2, size);
}

fn query_referral_settings(
    suite: &dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: u32,
) -> Option<ReferrerSettings> {
    suite
        .query_wasm_smart(perps, perps::QueryReferralSettingsRequest { user })
        .should_succeed()
}

fn query_referral_data(
    suite: &dango_testing::TestSuite<NaiveProposalPreparer>,
    perps: Addr,
    user: u32,
    since: Option<Timestamp>,
) -> UserReferralData {
    suite
        .query_wasm_smart(perps, perps::QueryReferralDataRequest { user, since })
        .should_succeed()
}
