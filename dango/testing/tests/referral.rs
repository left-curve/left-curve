use {
    dango_testing::{Factory, Preset, TestAccount, TestOption, TestSuite, setup_test},
    dango_types::{
        account_factory::{ExecuteMsg, QueryAccountRequest, RegisterUserData, UserIndex},
        config::AppConfig,
        constants::{eth, usdc},
        dex::{self, CreateOrderRequest, Price},
        oracle,
        taxman::{
            self, CommissionRebund, QueryConfigRequest, QueryReferralDataRequest,
            QueryReferralSettingsRequest, QueryReferrerRequest, ShareRatio, UserReferralData,
        },
    },
    grug::{
        Addr, Addressable, Bounded, Coin, Coins, Duration, HashExt, Inner, MakeBlockOutcome,
        Message, MultiplyFraction, NonEmpty, NonZero, Number, NumberConst, QuerierExt, ResultExt,
        Signer, Timestamp, TxOutcome, Udec128, Uint128, btree_map,
    },
    std::vec,
};

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
        query_latest_user_data(&mut suite, contracts.taxman, 1, None).referee_count,
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
        query_latest_user_data(&mut suite, contracts.taxman, 1, None).referee_count,
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
        query_latest_user_data(&mut suite, contracts.taxman, 1, None).referee_count,
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
        query_latest_user_data(&mut suite, contracts.taxman, 1, None).referee_count,
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
        query_latest_user_data(&mut suite, contracts.taxman, 2, None).referee_count,
        1
    );
}

#[test]
fn test_set_share_ratio() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    let mut cfg = suite
        .query_wasm_smart(contracts.taxman, QueryConfigRequest {})
        .should_succeed();

    // Set the required volume to $10k.
    cfg.referral.volume_to_be_referrer = Uint128::new(10_000 * 10_u128.pow(usdc::DECIMAL));

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
     "you must have at least a volume of 10000000000 USDC to become a referrer, traded volume: 0 USDC",
 );

    // Setup oracle and create a limit order with user1.
    {
        // Feed price to oracle.
        suite
     .execute(
         &mut accounts.owner,
         contracts.oracle,
         &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
             eth::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(1000), precision: 18, timestamp: Duration::from_weeks(1) },
             usdc::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(1), precision: 6, timestamp: Duration::from_weeks(1) },
         }),
         Coins::new(),
     )
     .should_succeed();

        // Create a ask limit order with user1.
        let amount = Uint128::new(10 * 10_u128.pow(eth::DECIMAL)); // 10 ETH
        let order = CreateOrderRequest::new_limit(
            eth::DENOM.clone(),
            usdc::DENOM.clone(),
            dex::Direction::Ask,
            NonZero::new(
                Price::checked_from_ratio(1000, 10_u128.pow(eth::DECIMAL - usdc::DECIMAL)).unwrap(),
            )
            .unwrap(),
            NonZero::new(amount).unwrap(),
        );

        suite
            .execute(
                &mut accounts.user1,
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![order],
                    cancels: None,
                },
                Coin::new(eth::DENOM.clone(), amount).unwrap(),
            )
            .should_succeed();
    }

    // Create a bid oreder with user2 of $9999. This is not enought to reach the required volume
    // to become a referrer.
    let usdc_amount = Uint128::new(9999 * 10_u128.pow(usdc::DECIMAL));
    create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

    set_share_ratio(
         &mut suite,
         contracts.taxman,
         &mut accounts.user2,
         Udec128::checked_from_ratio(5, 10).unwrap(),
     )
     .should_fail_with_error(
         "you must have at least a volume of 10000000000 USDC to become a referrer, traded volume: 9999000000 USDC",
     );

    // Trade another $1. Now the total traded volume is $10000, which should be enought for User2
    // to become a referrer.
    let usdc_amount = Uint128::new(10_u128.pow(usdc::DECIMAL));
    create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user2,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();
}

#[test]
fn modify_share_ratio() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    // The volume requirement to become a referrer is 0 by default.
    // Set share ratio for user1.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(2, 10).unwrap(),
    )
    .should_succeed();

    // Try to lower the share ratio (should fail).
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(1, 10).unwrap(),
    )
    .should_fail_with_error("you can only increase fee share ratio, current: 0.2, new: 0.1");

    // Increase the share ratio.
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(5, 10).unwrap(),
    )
    .should_succeed();

    // Try to increase the share ratio over the limit (should fail).
    set_share_ratio(
        &mut suite,
        contracts.taxman,
        &mut accounts.user1,
        Udec128::checked_from_ratio(6, 10).unwrap(),
    )
    .should_fail_with_error("fee share ratio cannot be higher than 0.5, new: 0.6");
}

#[test]
fn commission_rebound_tier() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    let mut taxman_config = suite
        .query_wasm_smart(contracts.taxman, QueryConfigRequest {})
        .should_succeed();

    // Set the commission rebound config.
    taxman_config.referral.commission_rebound_default =
        CommissionRebund::new_unchecked(Udec128::new_percent(10));

    taxman_config.referral.commission_rebound_by_volume = btree_map! {
        Udec128::new(100  * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(20)),
        Udec128::new(1000 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(30)),
        Udec128::new(2000 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(40)),
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman_config.clone(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Setup :
    // - oralcle feed price
    // - set share ratio for user1
    // - user1 create ask limit order
    // - user1 refer user2
    let user1_fee_share = Udec128::checked_from_ratio(2, 10).unwrap();
    let eth_human_price = 1000;

    {
        // Feed price to oracle.
        suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                eth::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(eth_human_price), precision: 18, timestamp: Duration::from_weeks(1) },
                usdc::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(1), precision: 6, timestamp: Duration::from_weeks(1) },
            }),
            Coins::new(),
        )
        .should_succeed();

        // Set share ratio for User1 to 20%.
        set_share_ratio(
            &mut suite,
            contracts.taxman,
            &mut accounts.user1,
            user1_fee_share,
        )
        .should_succeed();

        // Create a ask limit order with user1.
        let amount = Uint128::new(10 * 10_u128.pow(eth::DECIMAL)); // 10 ETH
        let order = CreateOrderRequest::new_limit(
            eth::DENOM.clone(),
            usdc::DENOM.clone(),
            dex::Direction::Ask,
            NonZero::new(
                Price::checked_from_ratio(1000, 10_u128.pow(eth::DECIMAL - usdc::DECIMAL)).unwrap(),
            )
            .unwrap(),
            NonZero::new(amount).unwrap(),
        );

        suite
            .execute(
                &mut accounts.user1,
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![order],
                    cancels: None,
                },
                Coin::new(eth::DENOM.clone(), amount).unwrap(),
            )
            .should_succeed();

        // user1 -> user2
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
    }

    // Since user1 is now a referrer, the commission rebound rate should be 10%.
    let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(10)
    );

    suite.block_time = Duration::from_days(1);

    // Make a bid order of 99$; this is not enought to reach the first tier, so the
    // commission rebound should still be 10%.
    {
        let usdc_amount = Uint128::new(99 * 10_u128.pow(usdc::DECIMAL));

        create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

        let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
        assert_eq!(
            referral_settings.commission_rebund.into_inner(),
            Udec128::new_percent(10)
        );
    }

    // Trade another 1$. This should be enough to reach the first tier,
    // so the commission rebound should now be 20%.
    {
        let usdc_amount = Uint128::new(10_u128.pow(usdc::DECIMAL));

        create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

        let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
        assert_eq!(
            referral_settings.commission_rebund.into_inner(),
            Udec128::new_percent(20)
        );
    }

    // Trade 1000$, should now reach 30% commission rebound.
    {
        let usdc_amount = Uint128::new(1000 * 10_u128.pow(usdc::DECIMAL));

        create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

        let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
        assert_eq!(
            referral_settings.commission_rebund.into_inner(),
            Udec128::new_percent(30)
        );
    }

    // Trade 1000$, should now reach 40% commission rebound.
    {
        let usdc_amount = Uint128::new(1000 * 10_u128.pow(usdc::DECIMAL));

        create_bid_order_block_outcome(&mut suite, contracts.dex, &mut accounts.user2, usdc_amount);

        let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
        assert_eq!(
            referral_settings.commission_rebund.into_inner(),
            Udec128::new_percent(40)
        );
    }

    // Go 29 days in the future. The total volume in the last 30 days should be only 1000$,
    // so the commission rebound should be back to 30%.
    suite.block_time = Duration::from_days(29);
    suite.make_block(vec![]);

    let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(30)
    );

    // Advance another day, now the volume in the last 30 days is 0, so the commission rebound rate should be back to 10%.
    suite.block_time = Duration::from_days(1);
    suite.make_block(vec![]);

    let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(10)
    );
}

#[test]
fn commission_rebound_coins() {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test(TestOption::preset_test());

    let mut taxman_config = suite
        .query_wasm_smart(contracts.taxman, QueryConfigRequest {})
        .should_succeed();

    let app_config: AppConfig = suite.query_app_config().should_succeed();

    // Set the commission rebound config.
    taxman_config.referral.commission_rebound_default =
        CommissionRebund::new_unchecked(Udec128::new_percent(10));

    taxman_config.referral.commission_rebound_by_volume = btree_map! {
        Udec128::new(200  * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(20)),
        Udec128::new(300 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(30)),
        Udec128::new(400 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(40)),
        Udec128::new(500 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(50)),
        Udec128::new(600 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(60)),
        Udec128::new(700 * 10_u128.pow(usdc::DECIMAL)) => CommissionRebund::new_unchecked(Udec128::new_percent(70)),
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.taxman,
            &taxman::ExecuteMsg::Configure {
                new_cfg: taxman_config.clone(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Setup :
    // - oralcle feed price
    // - set share ratio for user1
    // - user1 create ask limit order
    // - user1 refer user2
    let user1_fee_share = Udec128::checked_from_ratio(2, 10).unwrap();
    let eth_human_price = 1000;
    let eth_usdc_price =
        Udec128::checked_from_ratio(eth_human_price, 10_u128.pow(eth::DECIMAL - usdc::DECIMAL))
            .unwrap();

    let eth_unit_price =
        Udec128::checked_from_ratio(eth_human_price, 10_u128.pow(eth::DECIMAL)).unwrap();

    {
        // Feed price to oracle.
        suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                eth::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(eth_human_price), precision: 18, timestamp: Duration::from_weeks(1) },
                usdc::DENOM.clone() => oracle::PriceSource::Fixed { humanized_price: Udec128::new(1), precision: 6, timestamp: Duration::from_weeks(1) },
            }),
            Coins::new(),
        )
        .should_succeed();

        // Create a ask limit order with user1.
        let amount = Uint128::new(10 * 10_u128.pow(eth::DECIMAL)); // 10 ETH
        let order = CreateOrderRequest::new_limit(
            eth::DENOM.clone(),
            usdc::DENOM.clone(),
            dex::Direction::Ask,
            NonZero::new(
                Price::checked_from_ratio(1000, 10_u128.pow(eth::DECIMAL - usdc::DECIMAL)).unwrap(),
            )
            .unwrap(),
            NonZero::new(amount).unwrap(),
        );

        suite
            .execute(
                &mut accounts.user1,
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![order],
                    cancels: None,
                },
                Coin::new(eth::DENOM.clone(), amount).unwrap(),
            )
            .should_succeed();
    }

    let mut accounts_vec = vec![
        accounts.user1,
        accounts.user2,
        accounts.user3,
        accounts.user4,
        accounts.user5,
        accounts.user6,
        accounts.user7,
    ];

    // Set share ratio for all users.
    for account in &mut accounts_vec {
        set_share_ratio(&mut suite, contracts.taxman, account, user1_fee_share).should_succeed();
    }

    // Set referrer for all as:
    // user1 -> user2 -> user3 -> user4 -> user5 -> user6 -> user7
    for (i, account) in accounts_vec[1..].iter_mut().enumerate() {
        // user i + 1  -> user i + 2
        suite
            .execute(
                account,
                contracts.taxman,
                &taxman::ExecuteMsg::SetReferral {
                    referrer: (i + 1) as u32,
                    referee: (i + 2) as u32,
                },
                Coins::new(),
            )
            .should_succeed();
    }

    // The commission rebound is applied at the first five referrer, so if the user 7 make a trade,
    // all referrers except user1 should receive the commission rebound with the corresponding rate.

    // To test the fact that the referrer up to 5 receive the correct commission rebound, we will do a trade with the user7.
    // An indirect referrer receive some commission rebound only if his tax commission is greater than the maximum commission payed so far i
    // in the referrer chain,
    // e.g. user4 has commission rebound of 40%, user5 has commission rebound of 30% => user4 will receive 10% commission rebound;
    //      user4 has commission rebound of 30%, user5 has commission rebound of 30% => user4 will receive 0% commission rebound;

    // To test this correctly we will do like this:
    // user1 will have a commission rebound of 70%  # 6° referrer, should receive nothing
    // user2 will have a commission rebound of 60%  # 5° referrer, should receive 40% commission rebound
    // user3 will have a commission rebound of 10%  # 4° referrer, should receive nothing
    // user4 will have a commission rebound of 20%  # 3° referrer, should receive nothing
    // user5 will have a commission rebound of 20%  # 2° referrer, should receive 10% commission rebound
    // user6 will have a commission rebound of 10%  # 1° referrer, should receive 8% commission rebound, since it split with user7
    // user7 will have a commission rebound of 2%, since its fee share is 20% and the commission rebound rate is 10%.

    // Trade 200$ with user6 and user5 to have user5 and user4 to with 20% commission rebound.
    create_bid_order(
        &mut suite,
        contracts.dex,
        accounts_vec.get_mut(5).unwrap(), // user6
        Uint128::new(201 * 10_u128.pow(usdc::DECIMAL)),
    );

    let referrer_info = query_referral_settings(&suite, contracts.taxman, 5).unwrap();
    assert_eq!(
        referrer_info.commission_rebund.into_inner(),
        Udec128::new_percent(20)
    );

    create_bid_order(
        &mut suite,
        contracts.dex,
        accounts_vec.get_mut(4).unwrap(), // user5
        Uint128::new(200 * 10_u128.pow(usdc::DECIMAL)),
    );
    let referral_settings = query_referral_settings(&suite, contracts.taxman, 4).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(20)
    );

    // Trade with user3 to have user2 commission rebound at 60%.
    create_bid_order(
        &mut suite,
        contracts.dex,
        accounts_vec.get_mut(2).unwrap(), // user3
        Uint128::new(600 * 10_u128.pow(usdc::DECIMAL)),
    );

    let referral_settings = query_referral_settings(&suite, contracts.taxman, 2).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(60)
    );

    // Trade with user2 to have user1 commission rebound at 70%.
    create_bid_order_block_outcome(
        &mut suite,
        contracts.dex,
        accounts_vec.get_mut(1).unwrap(), // user2
        Uint128::new(700 * 10_u128.pow(usdc::DECIMAL)),
    );

    let referral_settings = query_referral_settings(&suite, contracts.taxman, 1).unwrap();
    assert_eq!(
        referral_settings.commission_rebund.into_inner(),
        Udec128::new_percent(70)
    );

    // Save the balances for all the referrer and the referee volumes and commission rebounds.
    let mut inital_balances = vec![];
    let mut inital_referral_data = vec![];

    for (i, account) in accounts_vec.iter().enumerate() {
        let user_index = i + 1;

        inital_balances.push(
            suite
                .query_balance(&account.address(), eth::DENOM.clone())
                .should_succeed(),
        );

        inital_referral_data.push(query_latest_user_data(
            &mut suite,
            contracts.taxman,
            user_index as u32,
            None,
        ));
    }

    // Trade 50$ with user7.
    let usdc_amount = Uint128::new(50 * 10_u128.pow(usdc::DECIMAL));
    create_bid_order_block_outcome(
        &mut suite,
        contracts.dex,
        accounts_vec.get_mut(6).unwrap(), // user7
        usdc_amount,
    );

    // Now we check that the each user received the correct commission rebound in coins.
    // We also check that the referral data is correctly updated.
    let order_data = calculate_bid_order_data(
        usdc_amount,
        eth_usdc_price,
        app_config.taker_fee_rate.into_inner(),
        Udec128::new_percent(10),
        Udec128::new_percent(20),
    );

    // User7.
    {
        let index = 6;
        let user7_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        assert_eq!(
            user7_balance,
            inital_balances[index] + order_data.eth_bought - order_data.trade_fee
                + order_data.commission_rebound_payer
        );

        let user7_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 7, None);

        assert_eq!(
            user7_referral_data.volume,
            inital_referral_data[index].volume + Udec128::new(usdc_amount.0)
        );

        assert_eq!(
            user7_referral_data.commission_rebounded,
            inital_referral_data[index].commission_rebounded
                + eth_unit_price
                    .checked_mul(Udec128::new(order_data.commission_rebound_payer.0))
                    .unwrap()
        );
    }

    // User6, 1° referrer, should receive 10% rebound fee.
    {
        let index = 5;
        let user6_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        assert_eq!(
            user6_balance,
            inital_balances[index] + order_data.commission_rebound_referrer
        );

        let user6_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 6, None);

        assert_eq!(
            user6_referral_data.referees_volume,
            inital_referral_data[index].referees_volume + Udec128::new(usdc_amount.0)
        );

        assert_eq!(
            user6_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
                + eth_unit_price
                    .checked_mul(Udec128::new(order_data.commission_rebound_referrer.0))
                    .unwrap()
        );
    }

    // User5, 2° referrer, should receive 10% rebound fee.
    {
        let index = 4;
        let user5_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        let user5_commission_rebound = order_data
            .trade_fee
            .checked_mul_dec_floor(Udec128::new_percent(10))
            .unwrap();

        assert_eq!(
            user5_balance,
            inital_balances[index] + user5_commission_rebound
        );

        let user5_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 5, None);

        // Referees volume is not updated for non direct referrer.
        assert_eq!(
            user5_referral_data.referees_volume,
            inital_referral_data[index].referees_volume,
        );

        assert_eq!(
            user5_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
                + eth_unit_price
                    .checked_mul(Udec128::new(user5_commission_rebound.0))
                    .unwrap()
        );
    }

    // User4, 3° referrer, should receive nothing.
    {
        let index = 3;
        let user4_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        assert_eq!(user4_balance, inital_balances[index]);

        let user4_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 4, None);

        // Referees volume is not updated for non direct referrer.
        assert_eq!(
            user4_referral_data.referees_volume,
            inital_referral_data[index].referees_volume,
        );

        assert_eq!(
            user4_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
        );
    }

    // User3, 4° referrer, should receive nothing.
    {
        let index = 2;
        let user3_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        assert_eq!(user3_balance, inital_balances[index]);

        let user3_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 3, None);

        // Referees volume is not updated for non direct referrer.
        assert_eq!(
            user3_referral_data.referees_volume,
            inital_referral_data[index].referees_volume,
        );

        assert_eq!(
            user3_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
        );
    }

    // User2, 5° referrer, should receive 40% rebound fee.
    {
        let index = 1;
        let user2_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        let user2_commission_rebound = order_data
            .trade_fee
            .checked_mul_dec_floor(Udec128::new_percent(40))
            .unwrap();

        assert_eq!(
            user2_balance,
            inital_balances[index] + user2_commission_rebound
        );

        let user2_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 2, None);

        assert_eq!(
            user2_referral_data.referees_volume,
            inital_referral_data[index].referees_volume
        );

        assert_eq!(
            user2_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
                + eth_unit_price
                    .checked_mul(Udec128::new(user2_commission_rebound.0))
                    .unwrap()
        );
    }

    // User1, 6° referrer, should receive nothing.
    {
        let index = 0;
        let user1_balance = suite
            .query_balance(&accounts_vec[index].address(), eth::DENOM.clone())
            .should_succeed();

        assert_eq!(user1_balance, inital_balances[index]);

        let user1_referral_data = query_latest_user_data(&mut suite, contracts.taxman, 1, None);

        // Referees volume is not updated for non direct referrer.
        assert_eq!(
            user1_referral_data.referees_volume,
            inital_referral_data[index].referees_volume,
        );

        assert_eq!(
            user1_referral_data.referees_commission_rebounded,
            inital_referral_data[index].referees_commission_rebounded
        );
    }
}

// Query the user data for a the referral contract.
fn query_latest_user_data(
    suite: &mut TestSuite,
    taxman: Addr,
    user: u32,
    since: Option<Timestamp>,
) -> UserReferralData {
    suite
        .query_wasm_smart(taxman, QueryReferralDataRequest { user, since })
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

// Create a bid order with the given amount and return the transaction outcome.
fn create_bid_order_block_outcome(
    suite: &mut TestSuite,
    dex: Addr,
    user: &mut TestAccount,
    amount: Uint128,
) -> MakeBlockOutcome {
    let order = CreateOrderRequest::new_market(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        dex::Direction::Bid,
        Bounded::new(Udec128::ZERO).unwrap(),
        NonZero::new(amount).unwrap(),
    );
    let msg = Message::execute(
        dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates: vec![order],
            cancels: None,
        },
        Coin::new(usdc::DENOM.clone(), amount).unwrap(),
    )
    .unwrap();

    let tx = user
        .sign_transaction(
            NonEmpty::new_unchecked(vec![msg]),
            &suite.chain_id,
            10000000000,
        )
        .unwrap();

    suite.make_block(vec![tx])
}

fn create_bid_order(suite: &mut TestSuite, dex: Addr, user: &mut TestAccount, amount: Uint128) {
    let order = CreateOrderRequest::new_market(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        dex::Direction::Bid,
        Bounded::new(Udec128::ZERO).unwrap(),
        NonZero::new(amount).unwrap(),
    );

    suite
        .execute(
            user,
            dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![order],
                cancels: None,
            },
            Coin::new(usdc::DENOM.clone(), amount).unwrap(),
        )
        .should_succeed();
}

fn calculate_bid_order_data(
    amount: Uint128,
    eth_usdc_price: Udec128,
    taker_fee_rate: Udec128,
    commission_rebound_rate: Udec128,
    fee_share: Udec128,
) -> BidOrderData {
    let eth_bought = amount.checked_div_dec_floor(eth_usdc_price).unwrap();

    let trade_fee = eth_bought.checked_mul_dec_floor(taker_fee_rate).unwrap();

    let commission_rebound_payer = trade_fee
        .checked_mul_dec_floor(commission_rebound_rate * fee_share)
        .unwrap();

    let commission_rebound_referrer = trade_fee
        .checked_mul_dec_floor(commission_rebound_rate * (Udec128::ONE - fee_share))
        .unwrap();

    BidOrderData {
        eth_bought,
        trade_fee,
        commission_rebound_payer,
        commission_rebound_referrer,
    }
}

struct BidOrderData {
    eth_bought: Uint128,
    trade_fee: Uint128,
    commission_rebound_payer: Uint128,
    commission_rebound_referrer: Uint128,
}

fn query_referral_settings(
    suite: &TestSuite,
    taxman: Addr,
    user: UserIndex,
) -> Option<taxman::ReferralSettings> {
    suite
        .query_wasm_smart(taxman, QueryReferralSettingsRequest { user })
        .should_succeed()
}
