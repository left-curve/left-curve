use {
    dango_testing::{
        HyperlaneTestSuite, TestOption, TestSuite,
        constants::{mock_ethereum, mock_solana},
        setup_test,
    },
    dango_types::{
        constants::{dango, usdc},
        gateway::{self, Origin, RateLimit, Remote},
    },
    grug::{
        Addr, BalanceChange, Coin, Coins, Duration, MathError, QuerierExt, ResultExt, Udec128,
        btree_map, btree_set, coins,
    },
    hyperlane_testing::MockValidatorSet,
    hyperlane_types::{Addr32, isms},
};

#[test]
fn rate_limit() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let mock_eth_recipient: Addr32 = Addr::mock(202).into();

    let usdc_sol_fee = 10_000;
    let usdc_eth_fee = 1_000_000;

    suite.balances().record(receiver);

    // Receive some tokens.
    // eth_usdc => 100
    // sol_usdc => 200
    {
        for (domain, origin_warp, amount) in [
            (mock_ethereum::DOMAIN, mock_ethereum::USDC_WARP, 100_000_000),
            (mock_solana::DOMAIN, mock_solana::USDC_WARP, 200_000_000),
        ] {
            suite
                .receive_warp_transfer(relayer, domain, origin_warp, receiver, amount)
                .should_succeed();
        }

        // Check balances.
        suite.balances().should_change(receiver, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Increased(300_000_000),
        });
    }

    suite
        .query_supply(usdc::DENOM.clone())
        .should_succeed_and_equal(300_000_000.into());

    // Total supply = 300 usdc
    // Set rate limit to 10%. This snapshots supply = 300 for the new denom.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Make 1 day pass — cron snapshots supply = 300, resets outbound = 0.
    advance_to_next_day(&mut suite);

    // Daily allowance = 300 * 10% = 30.
    // Send 30 alloy_usdc back to solana.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Trigger the rate limit sending 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("rate limit exceeded!");

    // Receive more tokens. The supply snapshot is fixed for the current window,
    // but the inbound credit (capped at daily allowance = 30) allows 30 more
    // outbound, so round-trips don't block other users.
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::USDC_WARP,
            receiver,
            100_000_000,
        )
        .should_succeed();

    // Supply = 300 - 30 + 100 = 370, snapshot still 300.
    // inbound = 100, credit = min(100, 30) = 30. Effective limit = 30 + 30 = 60.
    // Outbound is 30 — can withdraw 30 more thanks to inbound credit.
    {
        suite
            .query_supply(usdc::DENOM.clone())
            .should_succeed_and_equal(370_000_000.into());

        suite.balances().should_change(receiver, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Increased(369_990_000),
        });
    }

    // Withdraw 30 to ethereum — allowed by inbound credit.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_ethereum::DOMAIN,
                    contract: mock_ethereum::USDC_WARP,
                },
                recipient: mock_eth_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_eth_fee).unwrap(),
        )
        .should_succeed();

    // Inbound credit is capped at daily allowance (30). Outbound = 60, limit = 60.
    // 1 more token exceeds the cap.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_ethereum::DOMAIN,
                    contract: mock_ethereum::USDC_WARP,
                },
                recipient: mock_eth_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_eth_fee).unwrap(),
        )
        .should_fail_with_error("rate limit exceeded!");

    // Make 1 day pass — cron snapshots supply = 340, resets outbound and inbound.
    advance_to_next_day(&mut suite);

    // Daily allowance = 340 * 10% = 34.
    // Reserves: ETH = 200 - 30 = 170, SOL = 200 - 30 = 170.
    for (remote, amount) in [
        (
            Remote::Warp {
                domain: mock_ethereum::DOMAIN,
                contract: mock_ethereum::USDC_WARP,
            },
            170_000_000,
        ),
        (
            Remote::Warp {
                domain: mock_solana::DOMAIN,
                contract: mock_solana::USDC_WARP,
            },
            170_000_000,
        ),
    ] {
        suite
            .query_wasm_smart(contracts.gateway, gateway::QueryReserveRequest {
                bridge: contracts.warp,
                remote,
            })
            .should_succeed_and_equal(amount.into());
    }

    // Withdraw 34 tokens to solana (the full daily allowance).
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 34_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Try to withdraw 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("rate limit exceeded!");

    // Increase the rate limit to 99%.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(99)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Make 1 day pass — cron snapshots supply = 340 - 34 = 306, resets outbound.
    advance_to_next_day(&mut suite);

    // Daily allowance = 306 * 99% = 302 (floor).
    // SOL reserve = 170 - 34 = 136.

    // Withdraw all remaining SOL reserve.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 136_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // SOL reserve is now empty — should fail on reserve check.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient reserve!");
}

#[test]
fn native_denom() {
    let (mut suite, mut accounts, _, contracts, mut valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    let remote_domain = 123;
    let remote_warp = Addr::mock(123);

    // Register a native denom in the gateway
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.gateway,
                &gateway::ExecuteMsg::SetRoutes(btree_set!((
                    Origin::Local(dango::DENOM.clone(),),
                    contracts.warp,
                    Remote::Warp {
                        domain: remote_domain,
                        contract: remote_warp.into(),
                    }
                ))),
                Coins::default(),
            )
            .should_succeed();
    }

    // Register the validator set for the remote domain.
    {
        let validator_set = MockValidatorSet::new_preset(remote_domain, false);

        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.ism,
                &isms::multisig::ExecuteMsg::SetValidators {
                    domain: remote_domain,
                    threshold: 2,
                    validators: validator_set.validator_addresses(),
                },
                Coins::default(),
            )
            .should_succeed();

        valset.insert(remote_domain, validator_set);
    }

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    // Try receive a warp transfer.
    // This should fail because the gateway should not have any reserve for the native denom.
    {
        suite
            .receive_warp_transfer(
                &mut accounts.user3,
                remote_domain,
                remote_warp.into(),
                &accounts.user2,
                100,
            )
            .should_fail_with_error(MathError::overflow_sub(0_u128, 100_u128));
    }

    suite
        .balances()
        .record_many([&accounts.user1, &accounts.user2]);

    // Send some tokens with user1 to the remote domain.
    {
        suite
            .execute(
                &mut accounts.user1,
                contracts.gateway,
                &gateway::ExecuteMsg::TransferRemote {
                    remote: Remote::Warp {
                        domain: remote_domain,
                        contract: remote_warp.into(),
                    },
                    recipient: Addr::mock(124).into(),
                },
                coins! { dango::DENOM.clone() => 100 },
            )
            .should_succeed();
    }

    // Try to receive the tokens back to user2.
    {
        suite
            .receive_warp_transfer(
                &mut accounts.user3,
                remote_domain,
                remote_warp.into(),
                &accounts.user2,
                100,
            )
            .should_succeed();
    }

    // check the balances.
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Decreased(100),
    });

    suite.balances().should_change(&accounts.user2, btree_map! {
        dango::DENOM.clone() => BalanceChange::Increased(100),
    });
}

fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}
