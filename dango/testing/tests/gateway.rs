use {
    dango_testing::{
        HyperlaneTestSuite, TestOption, TestSuite,
        constants::{mock_ethereum, mock_solana},
        setup_test,
    },
    dango_types::{
        constants::{dango, usdc},
        gateway::{self, Origin, RateLimit, Remote, SetPersonalQuotaRequest},
    },
    grug_math::{MathError, NumberConst, Udec128, Uint128},
    grug_testing::BalanceChange,
    grug_types::{
        Addr, Addressable, Coin, Coins, Duration, Op, QuerierExt, ResultExt, btree_map, btree_set,
        coins,
    },
    hyperlane_testing::MockValidatorSet,
    hyperlane_types::{Addr32, isms},
};

#[tokio::test]
async fn rate_limit() {
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
                .await
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
    // Set rate limit.
    // alloy_usdc => 10%
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite).await;

    // Try send back exact tokens to don't trigger rate limit.
    // Current limit = 10% of 300 = 30
    // alloy_usdc => 300 * 0.1 = 30
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
        .await
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
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // Inflows must no longer replenish the outbound quota. Receive 100M more
    // USDC from Ethereum; the quota should stay at zero.
    suite
        .receive_warp_transfer(
            relayer,
            mock_ethereum::DOMAIN,
            mock_ethereum::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // Supply is now 300M + 100M = 400M minus the 30M already sent back to
    // solana = 370M. Receiver holds everything except the 10_000 fee paid.
    {
        suite
            .query_supply(usdc::DENOM.clone())
            .should_succeed_and_equal(370_000_000.into());

        suite.balances().should_change(receiver, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Increased(369_990_000),
        });
    }

    // Quota was not bumped by the inbound transfer — sending even 1 token
    // fails with the same error as before.
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
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // Advance one day so the cron seeds a fresh quota of 10% × 370M = 37M.
    advance_to_next_day(&mut suite).await;

    // Reserves: ethereum received 100M twice (no outflow) → 200M.
    //           solana received 200M, sent 30M back → 170M.
    for (remote, amount) in [
        (
            Remote::Warp {
                domain: mock_ethereum::DOMAIN,
                contract: mock_ethereum::USDC_WARP,
            },
            200_000_000,
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

    // Drain the full 37M quota to solana.
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
            Coin::new(usdc::DENOM.clone(), 37_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // One more token fails — quota is depleted and inflow can't refill it.
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
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // Raise the rate limit to 99%. In phase 1 this still only takes effect
    // after the next cron tick.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(99)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Another day. Supply is 370M - 37M = 333M; quota is 333M × 99%.
    advance_to_next_day(&mut suite).await;

    // Solana reserve after the previous 37M withdraw is 170M - 37M = 133M.
    // Drain it completely in a single transfer (well under the new quota).
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
            Coin::new(usdc::DENOM.clone(), 133_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Solana reserve is now empty; the next transfer fails on reserve, not
    // quota.
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
        .await
        .should_fail_with_error("insufficient reserve!");
}

#[tokio::test]
async fn boundary_attack() {
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

    // Mint 300M USDC into the chain via solana so the receiver has 300M to
    // send back over the same route, and the solana reserve can cover up to
    // 300M of outbound transfers.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            300_000_000,
        )
        .await
        .should_succeed();

    suite
        .query_supply(usdc::DENOM.clone())
        .should_succeed_and_equal(300_000_000.into());

    // Configure a 10% per-day rate limit. Cap = 30M against the 300M supply.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Cron tick: seeds the outbound quota at 30M.
    advance_to_next_day(&mut suite).await;

    // Advance to one minute before the next cron tick.
    advance_by(
        &mut suite,
        Duration::from_hours(23) + Duration::from_minutes(59),
    )
    .await;

    // Drain 25M, well under the 30M window cap.
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
            Coin::new(usdc::DENOM.clone(), 25_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Two minutes pass, crossing the cron tick. Supply is now 275M; cron
    // reseeds the cap to 10% × 275M = 27.5M.
    advance_by(&mut suite, Duration::from_minutes(2)).await;

    // Drain another 25M immediately after the cron tick. The trailing-24h
    // sum (25M from one minute earlier) plus the new 25M is 50M, exceeding
    // the 27.5M cap, so this is rejected.
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
            Coin::new(usdc::DENOM.clone(), 25_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 25000000, residue after personal quota: 25000000, rolling sum: 25000000, cap: 27500000");

    // Wait until the first drain falls outside the trailing 24h window. The
    // bucket from `t = 1d 23h59m` rolls out at `t = 2d 23h59m`; advance one
    // additional day (well past that boundary, also crossing another cron
    // tick) and confirm a 25M drain succeeds again.
    advance_by(&mut suite, Duration::from_hours(24)).await;

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
            Coin::new(usdc::DENOM.clone(), 25_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

#[tokio::test]
async fn native_denom() {
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
                &gateway::ExecuteMsg::SetRoutes(btree_set! {
                    (
                        Origin::Local(dango::DENOM.clone()),
                        contracts.warp,
                        Remote::Warp {
                            domain: remote_domain,
                            contract: remote_warp.into(),
                        },
                    ),
                }),
                Coins::default(),
            )
            .await
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
            .await
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
            .await
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
            .await
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
            .await
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

#[tokio::test]
async fn set_rate_limits_resets_quota() {
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

    // Receive 100M USDC from solana so we have reserves + supply to work with.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // Set a 10% rate limit. Supply is 100M, so the quota should be seeded to
    // 10M immediately — no cron tick needed.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Drain the full 10M quota.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 10_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Next token fails — the quota is now exhausted.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // Owner raises the rate limit to 50%. The change takes effect immediately
    // — the cap is `supply_snapshot × limit`, and only `limit` moved. The
    // snapshot (still 100M from the initial seed) yields cap = 50M. With 10M
    // already drained, 40M of headroom is now available in the same block.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(50)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // 1 more token now succeeds — admin's raise is honored immediately.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Advance one day so the cron fires and re-snapshots supply (~90M after
    // the 10M+1 drained). New cap = 90M × 50% = 45M; rolling sum reset to 0.
    advance_to_next_day(&mut suite).await;

    // A further 1-unit withdraw still succeeds — fresh headroom after the cron.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Removing USDC from the rate limits map should drop its cap entry. A
    // transfer far above the old 50% cap should now succeed — reserves are
    // the only remaining constraint.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {}),
            Coins::default(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 50_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Advance a day so the cron fires. The cron iterates RATE_LIMITS, which
    // no longer contains USDC, so no snapshot should be resurrected for it.
    // A subsequent large transfer must still succeed — reserves remain the
    // only constraint.
    advance_to_next_day(&mut suite).await;

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

/// `SetRateLimits` takes effect on the configured limit immediately, but
/// never refreshes the supply snapshot on a denom that is already tracked —
/// the snapshot only moves at cron ticks. Covers the no-op, lower, and raise
/// cases plus the cron-driven refresh.
#[tokio::test]
async fn set_rate_limits_does_not_refresh_supply_snapshot() {
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

    // Setup: 100M USDC supply, reserve 100M on solana.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // Seed a 50% rate limit. Snapshot = 100M, cap = 50M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(50)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Drain 30M → supply 70M, rolling sum 30M. Snapshot remains 100M.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Case 1: re-set the same limit. Snapshot is preserved (50M cap holds),
    // so headroom is 50M − 30M = 20M. 20M − 1 must succeed; 1 more must fail.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(50)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 19_999_999 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 2 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota!");

    // Case 2: lower the limit to 10%. Cap = 100M × 10% = 10M (snapshot still
    // 100M). Rolling sum is 49_999_999, already over the new 10M cap, so any
    // further withdraw fails.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota!");

    // Case 3: raise the limit to 80%. Cap = 100M × 80% = 80M (snapshot still
    // 100M, deposits between cron ticks do NOT enlarge it). Rolling sum is
    // 49_999_999, so headroom is ~30M and a 30M transfer must succeed.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(80)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Case 4: cron tick re-snapshots supply. Supply is now ~20M after total
    // ~80M drained; rolling sum carries through the 24h boundary in the
    // baseline calculation, so the new headroom comes from the fresh snapshot.
    advance_to_next_day(&mut suite).await;

    // A withdraw that would have failed pre-cron (cap was 80M, rolling sum
    // ~80M) now sees a smaller cap from the fresh snapshot. Any meaningful
    // amount above the fresh cap still fails; tiny amount succeeds against the
    // fresh headroom.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

#[tokio::test]
async fn personal_quota() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let owner_addr = accounts.owner.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // Seed 200M USDC from solana so there's both stock and a non-trivial
    // reserve for withdrawals to hit.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .await
        .should_succeed();

    // Tight 1% rate limit. Supply is 200M → global quota = 2M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(1)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // ---- Auth ----
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_fail_with_error("only the owner can set personal quotas");

    // ---- Overwrite + query ----
    // Initial 100M with a 1h lifetime.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(100_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Overwrite with a smaller, permanent allowance.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(50_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq.amount, Uint128::new(50_000_000));
    assert_eq!(pq.expire_at, None);
    assert_eq!(pq.granted_by, owner_addr);
    // Captured so the next assertion can check that partial consumption
    // preserves the grant's `granted_at`.
    let granted_at_after_overwrite = pq.granted_at;

    // ---- Consumption fully within personal quota ----
    // Personal = 50M, global = 2M. Withdraw 40M; only personal is touched.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 40_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq.amount, Uint128::new(10_000_000));
    assert_eq!(pq.expire_at, None);
    assert_eq!(pq.granted_by, owner_addr);
    // Partial consumption must preserve `granted_at` from the previous grant
    // — it's an audit field, not an activity timestamp.
    assert_eq!(pq.granted_at, granted_at_after_overwrite);

    // ---- Overflow into global quota ----
    // Withdraw 12M; personal (10M) is fully consumed, global absorbs 2M.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 12_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Fully consumed personal quotas are removed from storage.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(None);

    // Global is now depleted. The error mentions the remainder after any
    // personal quota would have been consumed — here that is the full amount.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // ---- Expired personal quota is ignored ----
    // Grant 100M more, this time with a 1h lifetime.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(100_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // 2h later — under the 24h cron interval, so the global quota stays at 0.
    advance_by(&mut suite, Duration::from_hours(2)).await;

    // The personal quota is now expired and must be skipped. Withdrawing 1
    // token falls through to the global quota (still 0) and fails.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, residue after personal quota: 1");

    // The expired entry is left in storage; the handler doesn't scrub it. The
    // caller can still query it to reason about `expire_at`.
    let stored = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed();
    assert_eq!(
        stored.as_ref().map(|q| q.amount),
        Some(Uint128::new(100_000_000))
    );
    assert!(stored.and_then(|q| q.expire_at).is_some());

    // ---- Pagination query ----
    let mut page = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotasRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();
    assert_eq!(page.len(), 1);
    let entry = page.pop().unwrap();
    assert_eq!(entry.user, receiver_addr);
    assert_eq!(entry.denom, usdc::DENOM.clone());
    assert_eq!(entry.quota.amount, Uint128::new(100_000_000));
}

/// `Op::Delete` must remove the personal quota entry outright — not just
/// flip its amount to zero — so subsequent withdrawals see no personal
/// allowance at all and fall straight to the global quota.
#[tokio::test]
async fn personal_quota_revoke_via_op_delete() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let owner_addr = accounts.owner.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // 100M supply. 10% rate limit → global quota = 10M.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Grant a 50M personal allowance.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(50_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq.amount, Uint128::new(50_000_000));
    assert_eq!(pq.expire_at, None);
    assert_eq!(pq.granted_by, owner_addr);

    // Revoke via `Op::Delete`.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Delete,
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Entry is gone — not just zeroed.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(None);

    // Try to withdraw 20M — above the 10M global quota. Pre-revocation the
    // personal quota would have covered it; post-revocation the transfer
    // must fall straight to the global quota and fail. The error message's
    // `remaining` equals the full request because no personal quota was
    // consumed.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 20_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error(
            "insufficient outbound quota! denom: bridge/usdc, requested: 20000000, residue after personal quota: 20000000",
        );

    // The 10M global quota still applies — 10M succeeds, 10M + 1 fails.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 10_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

/// A 0% rate limit is a hard freeze: the cap goes to zero AND every
/// outstanding personal quota for that denom is revoked, so a granted user
/// can't keep withdrawing through their per-account allowance.
#[tokio::test]
async fn zero_rate_limit_revokes_personal_quotas() {
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

    // 100M supply, 10% rate limit → cap 10M.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Grant the receiver a 5M personal quota for USDC.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver.address(),
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(5_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Confirm the quota is in place.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver.address(),
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|q| q.is_some());

    // Owner sets the USDC rate limit to 0 — a hard freeze.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::ZERO),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // The personal quota must have been wiped by the freeze.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver.address(),
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(None);

    // A 1-unit withdraw fails — no personal quota left and the global cap is 0.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota!");
}

/// Granting a personal quota for a denom that is NOT rate-limited globally
/// must still behave correctly: the personal allowance is consumed first,
/// and any overflow falls through to an absent global entry (which means
/// unrestricted, not "blocked").
#[tokio::test]
async fn personal_quota_on_un_rate_limited_denom() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let owner_addr = accounts.owner.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // Seed 100M reserve + supply. No SetRateLimits call anywhere — USDC is
    // not globally rate-limited.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // Grant a 50M personal allowance.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(50_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Consume 30M — fully from the personal quota.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq.amount, Uint128::new(20_000_000));
    assert_eq!(pq.expire_at, None);
    assert_eq!(pq.granted_by, owner_addr);

    // Withdraw 50M: 20M from personal (fully consumed), 30M falls through
    // to the global quota. Because the denom is not in RATE_LIMITS, the
    // fall-through finds no entry and the transfer succeeds unrestricted.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 50_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Personal quota is now fully consumed and removed from storage.
    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(None);

    // Without any personal or global restriction, a further transfer just
    // works. Reserves are the only remaining constraint.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 10_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

/// Overwriting a partially-consumed personal quota must replace the
/// record wholesale — no carry-over of the leftover balance, no
/// preservation of the old expiry. The stored amount and expiry reflect
/// the admin's most recent decision.
#[tokio::test]
async fn personal_quota_mid_consumption_overwrite() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let owner_addr = accounts.owner.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // Reserve / supply seed.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .await
        .should_succeed();

    // Tight global rate limit (1%) so the test leans on the personal quota.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(1)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Grant 100M with no expiry.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(100_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Consume 40M — fully within personal. 60M remains.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 40_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq.amount, Uint128::new(60_000_000));
    assert_eq!(pq.expire_at, None);
    assert_eq!(pq.granted_by, owner_addr);
    let granted_at_before_overwrite = pq.granted_at;

    // Overwrite: admin replaces with 10M + 1h expiry. The 60M leftover is
    // discarded; the expiry is newly computed from the current block time.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let stored = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present after overwrite");

    // Amount is the new 10M — no carry-over.
    assert_eq!(stored.amount, Uint128::new(10_000_000));
    // Expiry is Some — the new lifetime replaced the previous `None`.
    assert!(stored.expire_at.is_some());
    // Overwrites must reset `granted_at` — the record should reflect the
    // latest admin decision, not the first grant.
    assert!(stored.granted_at >= granted_at_before_overwrite);
    assert_eq!(stored.granted_by, owner_addr);
}

/// Paginated queries must return entries in ascending `(Addr, Denom)`
/// order and the `start_after` bound must correctly skip past the end of
/// the previous page.
#[tokio::test]
async fn personal_quotas_pagination() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let user_a = accounts.user1.address();
    let user_b = accounts.user2.address();
    let owner = &mut accounts.owner;

    // Grant four quotas across two users × two denoms.
    for (user, denom) in [
        (user_a, usdc::DENOM.clone()),
        (user_a, dango::DENOM.clone()),
        (user_b, usdc::DENOM.clone()),
        (user_b, dango::DENOM.clone()),
    ] {
        suite
            .execute(
                owner,
                contracts.gateway,
                &gateway::ExecuteMsg::SetPersonalQuota {
                    user,
                    denom,
                    quota: Op::Insert(SetPersonalQuotaRequest {
                        amount: Uint128::new(1_000_000),
                        available_for: None,
                    }),
                },
                Coins::default(),
            )
            .await
            .should_succeed();
    }

    // First page, limit 2.
    let page1 = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotasRequest {
            start_after: None,
            limit: Some(2),
        })
        .should_succeed();
    assert_eq!(page1.len(), 2);

    // Second page picks up after the last entry of page 1.
    let last = page1.last().expect("page 1 non-empty");
    let page2 = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotasRequest {
            start_after: Some((last.user, last.denom.clone())),
            limit: Some(2),
        })
        .should_succeed();
    assert_eq!(page2.len(), 2);

    // No entry overlap between pages.
    let page1_keys: std::collections::BTreeSet<_> =
        page1.iter().map(|e| (e.user, e.denom.clone())).collect();
    let page2_keys: std::collections::BTreeSet<_> =
        page2.iter().map(|e| (e.user, e.denom.clone())).collect();
    assert!(page1_keys.is_disjoint(&page2_keys));

    // Combined, the two pages cover exactly all four grants.
    let combined: std::collections::BTreeSet<_> = page1_keys.union(&page2_keys).cloned().collect();
    let expected: std::collections::BTreeSet<_> = [
        (user_a, usdc::DENOM.clone()),
        (user_a, dango::DENOM.clone()),
        (user_b, usdc::DENOM.clone()),
        (user_b, dango::DENOM.clone()),
    ]
    .into_iter()
    .collect();
    assert_eq!(combined, expected);

    // Each page is internally sorted ascending.
    assert!(
        page1
            .windows(2)
            .all(|w| (w[0].user, &w[0].denom) <= (w[1].user, &w[1].denom))
    );
    assert!(
        page2
            .windows(2)
            .all(|w| (w[0].user, &w[0].denom) <= (w[1].user, &w[1].denom))
    );

    // And the boundary between pages is also ascending.
    let last_p1 = page1.last().unwrap();
    let first_p2 = page2.first().unwrap();
    assert!((last_p1.user, &last_p1.denom) < (first_p2.user, &first_p2.denom));

    // Querying beyond the end yields an empty page.
    let last_p2 = page2.last().unwrap();
    let page3 = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotasRequest {
            start_after: Some((last_p2.user, last_p2.denom.clone())),
            limit: Some(2),
        })
        .should_succeed();
    assert!(page3.is_empty());
}

/// The `is_none_or(|t| block.timestamp < t)` predicate is strict. Cover
/// both sides of the boundary: at exactly `block.timestamp == expire_at`
/// the quota is already expired; 1ns before that it is still active.
#[tokio::test]
async fn personal_quota_expire_at_boundary() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // 100M supply; leave USDC un-rate-limited so that the transfer's quota
    // path depends only on the personal allowance.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // ---- Active: 1ns before expiry ----
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Advance to 1ns before the expiry. The predicate `now < expire_at` is
    // still true, so the personal quota is active.
    advance_by(
        &mut suite,
        Duration::from_hours(1) - Duration::from_nanos(1),
    )
    .await;

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // The active path consumed 1 token from the personal quota.
    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry still present");
    assert_eq!(pq.amount, Uint128::new(9_999_999));

    // ---- Expired: at exactly the boundary ----
    // Re-grant a fresh 10M with another 1h lifetime. `expire_at` is now
    // `current_block_time + 1h`; advancing by exactly 1h puts us right on
    // the boundary.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    advance_by(&mut suite, Duration::from_hours(1)).await;

    // The transfer should succeed (the denom is un-rate-limited), but the
    // personal quota must NOT be consumed — the predicate treats
    // `now == expire_at` as expired.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let pq = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("expired entry is left in storage untouched");
    assert_eq!(pq.amount, Uint128::new(10_000_000));
}

/// After a personal quota has expired (but before any consumption has
/// scrubbed it), re-granting must replace the stale entry cleanly —
/// fresh amount, fresh expire_at, fresh granted_at. No carry-over of the
/// old expired record.
#[tokio::test]
async fn personal_quota_regrant_after_expiry() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let owner_addr = accounts.owner.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;

    // Reserve + supply seed.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // Grant 10M with a 1h lifetime.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let pq_before = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    let granted_at_before = pq_before.granted_at;

    // Advance 2h so the entry is expired but has not been scrubbed by any
    // transfer attempt.
    advance_by(&mut suite, Duration::from_hours(2)).await;

    // Re-grant a fresh 20M with a new 1h lifetime. Under no carry-over, the
    // old expired record is replaced wholesale.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(20_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let pq_after = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");

    assert_eq!(pq_after.amount, Uint128::new(20_000_000));
    assert!(pq_after.expire_at.is_some());
    // expire_at was recomputed from the new block time, so it is strictly
    // later than the original expiry.
    assert!(pq_after.expire_at > pq_before.expire_at);
    assert_eq!(pq_after.granted_by, owner_addr);
    // granted_at was reset to the new block time, strictly later than the
    // original grant.
    assert!(pq_after.granted_at > granted_at_before);

    // The new allowance is actually usable — consume 1M and check the stored
    // amount drops to 19M (it's the new quota that's being consumed, not a
    // phantom merger with the old).
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let pq_consumed = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");
    assert_eq!(pq_consumed.amount, Uint128::new(19_000_000));
}

/// The cron only touches OUTBOUND_QUOTAS — it must never scrub
/// PERSONAL_QUOTAS, even if the entry is already expired. The expired
/// record should survive unchanged until the admin explicitly overwrites
/// or deletes it, or the user triggers consumption.
#[tokio::test]
async fn personal_quota_cron_tick_does_not_scrub_expired_entry() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    // Reserve + supply seed. (No mutable ref to `receiver` needed — the
    // test never transfers; it just grants and advances.)
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            &accounts.user2,
            100_000_000,
        )
        .await
        .should_succeed();

    // Global rate limit so cron_execute has something to reseed. This
    // isolates the cron's effect on OUTBOUND_QUOTAS from its effect on
    // PERSONAL_QUOTAS (which must be nil).
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Grant a 1h personal allowance.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc::DENOM.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(10_000_000),
                    available_for: Some(Duration::from_hours(1)),
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    let pq_before_cron = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("entry present");

    // Advance a full day. The personal quota expired 23h ago at this point.
    // The cron has fired at least once during this advance (24h tick).
    advance_to_next_day(&mut suite).await;

    let pq_after_cron = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed()
        .expect("expired entry is preserved across cron");

    // Every field of the expired record is untouched by cron.
    assert_eq!(pq_after_cron.amount, pq_before_cron.amount);
    assert_eq!(pq_after_cron.expire_at, pq_before_cron.expire_at);
    assert_eq!(pq_after_cron.granted_by, pq_before_cron.granted_by);
    assert_eq!(pq_after_cron.granted_at, pq_before_cron.granted_at);
}

/// Drains spread across the trailing window all count against the cap, but
/// each one falls out 24h after it was made.
#[tokio::test]
async fn rolling_window_releases_gradually() {
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

    // 200M USDC supply.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            200_000_000,
        )
        .await
        .should_succeed();

    // 5% rate limit. Cap = 10M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(5)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Drain the full 10M cap immediately.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 10_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // One minute before the 24h boundary, the drain is still in the window.
    advance_by(
        &mut suite,
        Duration::from_hours(23) + Duration::from_minutes(59),
    )
    .await;

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_fail_with_error("insufficient outbound quota!");

    // Cross 24h since the drain (and the cron tick at 1d). The original
    // entry has rolled out; the cron has reseeded the cap to 9.5M (190M ×
    // 5%). A fresh full-cap drain succeeds.
    advance_by(&mut suite, Duration::from_minutes(2)).await;

    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc::DENOM.clone(), 9_500_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();
}

/// The cap is snapshotted by cron once per refresh period — supply changes
/// between cron ticks (deposits, etc.) do not enlarge the headroom.
#[tokio::test]
async fn cap_is_snapshotted_at_cron_tick() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let usdc_denom = usdc::DENOM.clone();

    // Receive 100M USDC. supply = 100M.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // 10% rate limit. Snapshot seeded at supply = 100M, so cap = 10M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    let initial = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(initial.cap, Uint128::new(10_000_000));
    assert_eq!(initial.used_in_last_24h, Uint128::ZERO);

    // Receive another 100M. supply = 200M, but cap stays at 10M until cron.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    let mid = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(mid.cap, Uint128::new(10_000_000));
    assert_eq!(mid.used_in_last_24h, Uint128::ZERO);

    // Cron tick reseeds the cap to 200M × 10% = 20M.
    advance_to_next_day(&mut suite).await;

    let after_cron = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom,
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(after_cron.cap, Uint128::new(20_000_000));
}

/// A withdraw fully covered by personal quota does not consume the trailing
/// rolling window — the global cap stays available for other withdraws.
#[tokio::test]
async fn personal_quota_does_not_consume_rolling_window() {
    let (mut suite, mut accounts, _, contracts, valset) = setup_test(TestOption {
        bridge_ops: |_| vec![],
        ..TestOption::default()
    });

    suite.block_time = Duration::ZERO;

    let mut suite = HyperlaneTestSuite::new(suite, valset, &contracts);

    let receiver_addr = accounts.user2.address();
    let receiver = &mut accounts.user2;
    let relayer = &mut accounts.user1;
    let owner = &mut accounts.owner;

    let mock_solana_recipient: Addr32 = Addr::mock(201).into();
    let usdc_sol_fee = 10_000;
    let usdc_denom = usdc::DENOM.clone();

    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    // 1% rate limit. Cap = 1M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(1)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Grant a 50M personal quota — large enough to fully cover the test
    // withdraw without spilling into the global cap.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetPersonalQuota {
                user: receiver_addr,
                denom: usdc_denom.clone(),
                quota: Op::Insert(SetPersonalQuotaRequest {
                    amount: Uint128::new(50_000_000),
                    available_for: None,
                }),
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Withdraw 40M, fully within the personal allowance.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc_denom.clone(), 40_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // The trailing-window sum is still zero.
    let q = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom,
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(q.used_in_last_24h, Uint128::ZERO);
    assert_eq!(q.cap, Uint128::new(1_000_000));
}

/// Removing a denom from the rate-limit map clears its trailing-window
/// state. Re-adding it later starts with a fresh rolling sum.
#[tokio::test]
async fn denom_removal_clears_withdraw_volumes() {
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
    let usdc_denom = usdc::DENOM.clone();

    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(20)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Drain 5M — rolling sum is now 5M.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc_denom.clone(), 5_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    let after_drain = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(after_drain.used_in_last_24h, Uint128::new(5_000_000));

    // Drop USDC from the rate-limit map. The cap entry and rolling-window
    // history should both go away.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {}),
            Coins::default(),
        )
        .await
        .should_succeed();

    let unlimited = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed();
    assert!(unlimited.is_none());

    // Re-add USDC at a fresh rate. supply has dropped to 95M, cap = 19M.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(20)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    let reseeded = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom,
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(reseeded.cap, Uint128::new(19_000_000));
    assert_eq!(reseeded.used_in_last_24h, Uint128::ZERO);
}

/// Exercise the `RateLimitStatus` and paginated `RateLimitStatuses` queries.
#[tokio::test]
async fn query_rate_limit_status() {
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
    let usdc_denom = usdc::DENOM.clone();

    // Un-rate-limited denom returns None.
    let none = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed();
    assert!(none.is_none());

    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
        .await
        .should_succeed();

    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Drain 3M to leave a non-trivial rolling sum.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: mock_solana::DOMAIN,
                    contract: mock_solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            },
            Coin::new(usdc_denom.clone(), 3_000_000 + usdc_sol_fee).unwrap(),
        )
        .await
        .should_succeed();

    // Single-denom query.
    let single = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom.clone(),
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(single.supply_snapshot, Uint128::new(100_000_000));
    assert_eq!(single.cap, Uint128::new(10_000_000));
    assert_eq!(single.used_in_last_24h, Uint128::new(3_000_000));

    // Paginated enumeration returns the same data.
    let page = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();
    assert_eq!(page.len(), 1);
    let status = page.get(&usdc_denom).expect("usdc rate-limited");
    assert_eq!(status.supply_snapshot, Uint128::new(100_000_000));
    assert_eq!(status.cap, Uint128::new(10_000_000));
    assert_eq!(status.used_in_last_24h, Uint128::new(3_000_000));

    // After 24h + cron, the rolling sum drops back to zero.
    advance_to_next_day(&mut suite).await;

    let aged = suite
        .query_wasm_smart(contracts.gateway, gateway::QueryRateLimitStatusRequest {
            denom: usdc_denom,
        })
        .should_succeed()
        .expect("rate-limited");
    assert_eq!(aged.used_in_last_24h, Uint128::ZERO);
}

async fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block().await;
    suite.block_time = Duration::ZERO;
}

async fn advance_by(suite: &mut TestSuite, d: Duration) {
    suite.block_time = d;
    suite.make_empty_block().await;
    suite.block_time = Duration::ZERO;
}
