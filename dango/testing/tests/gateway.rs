use {
    dango_testing::{
        HyperlaneTestSuite, TestOption, TestSuite,
        constants::{mock_ethereum, mock_solana},
        setup_test,
    },
    dango_types::{
        bank,
        constants::{dango, eth, usdc},
        gateway::{self, Origin, RateLimit, Remote},
    },
    grug::{
        Addr, BalanceChange, Coin, Coins, Duration, MathError, NumberConst, QuerierExt, ResultExt,
        Udec128, Uint128, btree_map, btree_set, coins,
    },
    hyperlane_testing::MockValidatorSet,
    hyperlane_types::{Addr32, isms},
};

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

/// Verify global rate limit enforcement: withdraw up to the max, then 1 more
/// fails. After a deposit, the user can withdraw up to the deposited amount
/// without hitting the global limit. After a new epoch, limits reset.
#[test]
fn rate_limit_global_enforcement() {
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
    let usdc_sol_fee = 10_000;

    // Deposit 200 USDC from Solana.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .should_succeed();

    // Set rate limit to 10%. Supply = 200, daily allowance = 20.
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

    // Verify initial state.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryEpochRequest {})
        .should_succeed_and_equal(0_u64);

    suite
        .query_wasm_smart(contracts.gateway, gateway::QuerySupplyRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(200_000_000.into());

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryAvailableWithdrawRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(Uint128::new(20_000_000)));

    // Advance to epoch 24 — deposits from epoch 0 rotate to historical,
    // current epoch has no deposit credit.
    advance_to_next_day(&mut suite);

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryEpochRequest {})
        .should_succeed_and_equal(24_u64);

    // User has no deposit credit this epoch, so available = global = 20.
    suite
        .query_wasm_smart(
            contracts.gateway,
            gateway::QueryUserAvailableWithdrawRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            },
        )
        .should_succeed_and_equal(Some(Uint128::new(20_000_000)));

    // Withdraw the full daily allowance (20).
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
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Global available = 0 (20 - 20). User available = 0 (no credit).
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryAvailableWithdrawRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(Uint128::ZERO));

    suite
        .query_wasm_smart(
            contracts.gateway,
            gateway::QueryUserAvailableWithdrawRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            },
        )
        .should_succeed_and_equal(Some(Uint128::ZERO));

    // Verify user movement state.
    {
        let user_mov: gateway::UserMovement = suite
            .query_wasm_smart(contracts.gateway, gateway::QueryUserMovementRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            })
            .unwrap();
        assert_eq!(user_mov.current.deposited, Uint128::ZERO);
        assert_eq!(user_mov.current.withdrawn, Uint128::new(20_000_000));
        assert_eq!(user_mov.current.credit_used, Uint128::ZERO);
        assert_eq!(user_mov.historical.deposited, Uint128::new(200_000_000));
    }

    // 1 more fails — global limit reached.
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

    // Deposit 50 more in this epoch — gives deposit credit.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            50_000_000,
        )
        .should_succeed();

    // Global still 0, but user now has 50 credit → user available = 50.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryAvailableWithdrawRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(Uint128::ZERO));

    suite
        .query_wasm_smart(
            contracts.gateway,
            gateway::QueryUserAvailableWithdrawRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            },
        )
        .should_succeed_and_equal(Some(Uint128::new(50_000_000)));

    // Deposited 50 this epoch. Previous 20 withdrawal was charged to global,
    // NOT to credit. So remaining credit = 50. Withdraw all 50.
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
            Coin::new(usdc::DENOM.clone(), 50_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Credit exhausted. Verify state.
    {
        let user_mov: gateway::UserMovement = suite
            .query_wasm_smart(contracts.gateway, gateway::QueryUserMovementRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            })
            .unwrap();
        assert_eq!(user_mov.current.deposited, Uint128::new(50_000_000));
        assert_eq!(user_mov.current.withdrawn, Uint128::new(70_000_000));
        assert_eq!(user_mov.current.credit_used, Uint128::new(50_000_000));
    }

    // Global still 20 (the 50 was within credit). User available = 0.
    suite
        .query_wasm_smart(
            contracts.gateway,
            gateway::QueryUserAvailableWithdrawRequest {
                user_index: receiver.user_index(),
                denom: usdc::DENOM.clone(),
            },
        )
        .should_succeed_and_equal(Some(Uint128::ZERO));

    // 1 more is excess → global already at 20, so 20 + 1 > 20 → fails.
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

    // Advance to epoch 48. Supply = 200 - 20 + 50 - 50 = 180. Allowance = 18.
    advance_to_next_day(&mut suite);

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryEpochRequest {})
        .should_succeed_and_equal(48_u64);

    suite
        .query_wasm_smart(contracts.gateway, gateway::QuerySupplyRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(180_000_000.into());

    // Global outbound rolled off — fresh window.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryAvailableWithdrawRequest {
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(Uint128::new(18_000_000)));

    // Fresh epoch — global outbound is 0 again. Withdraw 18 succeeds.
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
            Coin::new(usdc::DENOM.clone(), 18_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // 1 more fails again.
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
}

/// Verify per-user deposit credit: if one user exhausts the global limit,
/// another user who deposited in the same epoch can still withdraw up to their
/// deposit amount.
#[test]
fn rate_limit_per_user_deposit_credit() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let user_a = &mut accounts.user1;
    let user_b = &mut accounts.user2;
    let relayer = &mut accounts.user3;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // Deposit 100 to user_a and 100 to user_b.
    for user in [&*user_a, &*user_b] {
        suite
            .receive_warp_transfer(
                relayer,
                mock_solana::DOMAIN,
                mock_solana::USDC_WARP,
                user,
                100_000_000,
            )
            .should_succeed();
    }

    // Set rate limit 10%. Supply = 200, daily allowance = 20.
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

    advance_to_next_day(&mut suite);

    // Deposit 50 to user_b in the new epoch — gives user_b deposit credit.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            &*user_b,
            50_000_000,
        )
        .should_succeed();

    // user_a exhausts the global limit (20).
    suite
        .execute(
            user_a,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // user_a can't withdraw any more.
    suite
        .execute(
            user_a,
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

    // user_b can still withdraw up to their deposit credit (50), even though
    // the global limit is exhausted — deposit-backed withdrawals are free.
    suite
        .execute(
            user_b,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 50_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // user_b's credit is exhausted. 1 more is excess → global already at 20 → fails.
    suite
        .execute(
            user_b,
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
}

/// Verify deposit credit is denom-wide, not per-route: a deposit from ETH
/// gives credit that can be used to withdraw via SOL, and vice versa.
#[test]
fn rate_limit_across_routes() {
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

    // Deposit 60 from ETH and 40 from SOL = 100 total.
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::USDC_WARP,
            receiver,
            60_000_000,
        )
        .should_succeed();
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            40_000_000,
        )
        .should_succeed();

    // Set rate limit 10%. Supply = 100, daily allowance = 10.
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

    advance_to_next_day(&mut suite);

    // Deposit 20 from ETH in this epoch → credit = 20.
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::USDC_WARP,
            receiver,
            20_000_000,
        )
        .should_succeed();

    // Withdraw 30 via SOL route — within credit (ETH deposit covers SOL
    // withdrawal because credit is per-denom, not per-route). Credit = 0.
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
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Credit exhausted. Global outbound is still 0 (all within credit).
    // Withdraw 10 via SOL → excess = 10. Global: 0 + 10 = 10 ≤ 10. Succeeds.
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
            Coin::new(usdc::DENOM.clone(), 10_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // 1 more via either route → excess → global full → fails.
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
}

/// Verify that per-route reserves are respected: a user cannot withdraw more
/// from a specific route than what was deposited through that route, even if
/// they have deposit credit and global allowance available.
#[test]
fn rate_limit_reserve_enforcement() {
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

    // Deposit 80 from ETH and 20 from SOL = 100 total.
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::USDC_WARP,
            receiver,
            80_000_000,
        )
        .should_succeed();
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            20_000_000,
        )
        .should_succeed();

    // Set rate limit 99% so it doesn't interfere — we're testing reserves.
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

    // Check reserves: ETH = 80, SOL = 20.
    for (remote, amount) in [
        (
            Remote::Warp {
                domain: mock_ethereum::DOMAIN,
                contract: mock_ethereum::USDC_WARP,
            },
            80_000_000,
        ),
        (
            Remote::Warp {
                domain: mock_solana::DOMAIN,
                contract: mock_solana::USDC_WARP,
            },
            20_000_000,
        ),
    ] {
        suite
            .query_wasm_smart(contracts.gateway, gateway::QueryReserveRequest {
                bridge: contracts.warp,
                remote,
            })
            .should_succeed_and_equal(amount.into());
    }

    // Withdraw all 20 from SOL reserve.
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
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // SOL reserve is now 0. Trying to withdraw 1 more via SOL fails on reserve,
    // even though the user has credit and ETH reserve is still 80.
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

    // Withdrawing via ETH still works — ETH reserve is 80.
    // Withdraw 70 (balance is ~79.99 after SOL withdrawal fees).
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
            Coin::new(usdc::DENOM.clone(), 70_000_000 + usdc_eth_fee).unwrap(),
        )
        .should_succeed();

    // ETH reserve = 80 - 70 = 10.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryReserveRequest {
            bridge: contracts.warp,
            remote: Remote::Warp {
                domain: mock_ethereum::DOMAIN,
                contract: mock_ethereum::USDC_WARP,
            },
        })
        .should_succeed_and_equal(10_000_000.into());

    // SOL reserve is still 0 — can't withdraw via SOL.
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

/// Verify that changing the rate limit takes effect immediately: increase it to
/// unlock more withdrawals, then decrease it to block them again.
#[test]
fn rate_limit_change() {
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
    let usdc_sol_fee = 10_000;

    // Deposit 200 USDC.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .should_succeed();

    // Set rate limit to 10%. Supply = 200, daily allowance = 20.
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

    advance_to_next_day(&mut suite);

    // Exhaust the daily allowance (20).
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
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // 1 more fails.
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

    // --- Increase rate limit to 50%. ---
    // Supply snapshot is still 200 (no cron), daily allowance = 200 * 50% = 100.
    // Global outbound = 20 (from before). 100 - 20 = 80 remaining.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(50)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Now 80 more can be withdrawn. Withdraw 80.
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
            Coin::new(usdc::DENOM.clone(), 80_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Global outbound = 100. 1 more fails.
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

    // --- Lower rate limit to 5%. ---
    // Daily allowance = 200 * 5% = 10. But global outbound is already 100.
    // 100 > 10 → all further withdrawals are blocked immediately.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(5)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Even 1 token fails — outbound (100) already far exceeds new allowance (10).
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

    // After a new epoch, outbound resets and the lower limit applies.
    // Supply = 200 - 20 - 80 = 100. Daily allowance = 100 * 5% = 5.
    advance_to_next_day(&mut suite);

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
            Coin::new(usdc::DENOM.clone(), 5_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

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
}

/// Verify that deposit credit is tracked independently per denom: depositing
/// denom A does not grant credit for withdrawing denom B. Before the fix
/// (keying `USER_MOVEMENTS` by `(UserIndex, &Denom)` instead of `UserIndex`),
/// this test would fail because all deposit credit was pooled across denoms.
#[test]
fn rate_limit_per_denom_isolation() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_eth_recipient: Addr32 = Addr::mock(202).into();
    let usdc_sol_fee = 10_000;
    let eth_fee = 500_000_000_000_000_u128;

    // Deposit 200 USDC and 10 ETH (10_000_000_000_000_000_000 = 10e18).
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .should_succeed();
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::ETH_WARP,
            receiver,
            10_000_000_000_000_000_000,
        )
        .should_succeed();

    // Set rate limits: USDC 10%, ETH 10%.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
                eth::DENOM.clone()  => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Advance to epoch 1 so the epoch-0 deposits rotate away — no credit
    // from them in the new epoch.
    advance_to_next_day(&mut suite);

    // USDC: supply = 200, daily allowance = 20.
    // ETH:  supply = 10 ETH, daily allowance = 1 ETH (1_000_000_000_000_000_000).

    // Exhaust the USDC daily allowance (20).
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: Addr::mock(201).into(),
            },
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // USDC limit is exhausted — 1 more fails.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: Addr::mock(201).into(),
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("rate limit exceeded!");

    // Now deposit 50 USDC in this epoch — gives USDC-specific credit of 50.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            50_000_000,
        )
        .should_succeed();

    // The USDC credit must NOT help with ETH withdrawals.
    // ETH daily allowance = 1 ETH. Withdraw 1 ETH (the full allowance).
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_ethereum::DOMAIN,
                    contract: mock_ethereum::ETH_WARP,
                },
                recipient: mock_eth_recipient,
            },
            Coin::new(eth::DENOM.clone(), 1_000_000_000_000_000_000_u128 + eth_fee).unwrap(),
        )
        .should_succeed();

    // ETH global limit is exhausted. 1 more wei fails.
    // With the old per-user-only key, the 50 USDC deposit credit would have
    // been fungible and this withdrawal would have incorrectly succeeded.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_ethereum::DOMAIN,
                    contract: mock_ethereum::ETH_WARP,
                },
                recipient: mock_eth_recipient,
            },
            Coin::new(eth::DENOM.clone(), 1_u128 + eth_fee).unwrap(),
        )
        .should_fail_with_error("rate limit exceeded!");

    // Meanwhile the USDC credit is intact — can still withdraw 50 USDC.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: gateway::Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: Addr::mock(201).into(),
            },
            Coin::new(usdc::DENOM.clone(), 50_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();
}

/// Verify the sliding window prevents the boundary double-dip attack: withdraw
/// max at the end of one hour, advance one hourly epoch, then try again — the
/// rolling 24h total still includes the previous withdrawal.
#[test]
fn rate_limit_boundary_attack() {
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
    let usdc_sol_fee = 10_000;

    // Deposit 240 USDC.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            240_000_000,
        )
        .should_succeed();

    // Set rate limit 10%. Supply = 240, daily allowance = 24.
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

    advance_to_next_day(&mut suite);

    // Withdraw the full daily allowance (24).
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
            Coin::new(usdc::DENOM.clone(), 24_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Advance only 1 hourly epoch (not a full day).
    advance_one_hour(&mut suite);

    // The attacker tries to withdraw again. The sliding window still contains
    // the previous 24 from the last hour. Rolling outbound = 24.
    // Daily allowance = 24. So 24 + 1 > 24 → fails.
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

    // After a full day (24 hours), the old withdrawal has fully dropped off.
    // Advance remaining 23 hours.
    for _ in 0..23 {
        advance_one_hour(&mut suite);
    }

    // Now the rolling window is empty. Supply was recalculated at epoch 48
    // (24 + 24). Supply = 240 - 24 = 216. Daily allowance = 216 * 10% = 21.6.
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
            Coin::new(usdc::DENOM.clone(), 21_600_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // 1 more fails.
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
}

/// Verify that depositing to an unregistered address succeeds — the funds are
/// held as an orphan transfer in the bank.
#[test]
fn deposit_to_unregistered_address() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let relayer = &mut accounts.user1;

    let unregistered = Addr::mock(250);

    // Deposit 50 USDC to an unregistered address — should succeed.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            &unregistered,
            50_000_000,
        )
        .should_succeed();

    // The funds should be held as an orphan transfer in the bank, with the
    // gateway contract as the sender.
    suite
        .query_wasm_smart(
            contracts.bank,
            bank::QueryOrphanedTransfersByRecipientRequest {
                recipient: unregistered,
                start_after: None,
                limit: None,
            },
        )
        .should_succeed_and_equal(btree_map! {
            contracts.gateway => coins! { usdc::DENOM.clone() => 50_000_000 },
        });
}

/// Verify that a zero rate limit acts as an emergency freeze: all withdrawals
/// are blocked, even deposit-credited ones. Unfreezing (setting a non-zero
/// rate limit) re-enables withdrawals.
#[test]
fn rate_limit_zero_freeze() {
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
    let usdc_sol_fee = 10_000;

    // Deposit 100 USDC.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .should_succeed();

    // Set rate limit to 0% — emergency freeze.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::ZERO),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Even though the user has 100 of deposit credit in the same epoch,
    // a zero rate limit blocks everything.
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
        .should_fail_with_error("withdrawals are frozen");

    // Unfreeze by setting a non-zero rate limit.
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

    // Withdrawals work again — deposit credit covers this.
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
        .should_succeed();
}

/// Advance 24 hourly epochs (one full rate-limit day).
fn advance_to_next_day(suite: &mut TestSuite) {
    for _ in 0..24 {
        advance_one_hour(suite);
    }
}

/// Advance 1 hourly epoch (triggers cron_execute once).
fn advance_one_hour(suite: &mut TestSuite) {
    suite.block_time = Duration::from_hours(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}
