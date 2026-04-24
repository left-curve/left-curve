use {
    dango_testing::{
        HyperlaneTestSuite, TestOption, TestSuite,
        constants::{mock_ethereum, mock_solana},
        setup_test,
    },
    dango_types::{
        constants::{dango, usdc},
        gateway::{self, Origin, PersonalQuota, RateLimit, Remote, SetPersonalQuotaRequest},
    },
    grug::{
        Addr, Addressable, BalanceChange, Coin, Coins, Duration, MathError, Op, QuerierExt,
        ResultExt, Udec128, Uint128, btree_map, btree_set, coins,
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
        .should_succeed();

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

    // Advance one day so the cron seeds a fresh quota of 10% × 370M = 37M.
    advance_to_next_day(&mut suite);

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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

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
        .should_succeed();

    // Another day. Supply is 370M - 37M = 333M; quota is 333M × 99%.
    advance_to_next_day(&mut suite);

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

#[test]
fn set_rate_limits_resets_quota() {
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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

    // Owner raises the rate limit to 50% without advancing time. Raising
    // must NOT take effect mid-window — otherwise a well-timed SetRateLimits
    // call lets the same user drain `supply × limit` twice back-to-back in
    // the same 24-hour window (once now, once after the next cron tick).
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

    // Quota is still 0 (raise is deferred to next cron). 1 more token fails.
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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

    // Advance one day so the cron fires and reseeds. Supply is 90M (after
    // the 10M drain), so the new quota is 45M.
    advance_to_next_day(&mut suite);

    // The call that failed above now succeeds — quota has 45M headroom.
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
        .should_succeed();

    // Removing USDC from the rate limits map should drop its entry in
    // OUTBOUND_QUOTAS. A transfer far above the old 50% quota should now
    // succeed — reserves are the only remaining constraint.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {}),
            Coins::default(),
        )
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
        .should_succeed();

    // Advance a day so the cron fires. The cron iterates RATE_LIMITS, which
    // no longer contains USDC, so no OUTBOUND_QUOTAS entry should be
    // resurrected for it. A subsequent large transfer must still succeed —
    // reserves remain the only constraint.
    advance_to_next_day(&mut suite);

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
        .should_succeed();
}

/// `SetRateLimits` must never refill a partially-drained outbound quota
/// mid-window — otherwise a well-timed admin call right before the next
/// cron tick lets the same user drain `supply × limit` twice in a row.
/// Covers the four cases the tighten helper has to get right: no-op,
/// lower, raise, and cron reseed.
#[test]
fn set_rate_limits_only_tightens_existing_quotas() {
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
        .should_succeed();

    // Seed a 50% rate limit. Quota = 50M.
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

    // Drain 30M → quota 20M, supply 70M.
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
        .should_succeed();

    // Case 1: same limit re-set. min(20M, 70M × 50% = 35M) = 20M. No change
    // — the drained state is preserved.
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

    // Transfer 20M + 1 must still fail. If the same-limit call had reset the
    // quota to `supply × 50% = 35M`, this would succeed.
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
            Coin::new(usdc::DENOM.clone(), 20_000_001 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient outbound quota!");

    // Case 2: lower the limit to 10%. Tightens to min(20M, 70M × 10% = 7M)
    // = 7M — takes effect immediately.
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

    // 7M + 1 fails, 7M succeeds.
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
            Coin::new(usdc::DENOM.clone(), 7_000_001 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient outbound quota!");

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
            Coin::new(usdc::DENOM.clone(), 7_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Quota is now 0. Supply 63M.

    // Case 3: raise the limit to 80%. Would be `63M × 80% = 50.4M` if the
    // admin call reseeded, but tighten-only preserves the drained 0.
    suite
        .execute(
            owner,
            contracts.gateway,
            &gateway::ExecuteMsg::SetRateLimits(btree_map! {
                usdc::DENOM.clone() => RateLimit::new_unchecked(Udec128::new_percent(80)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Even 1 more token fails — no quota has been freed by the raise.
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
        .should_fail_with_error("insufficient outbound quota!");

    // Case 4: cron tick reseeds to the raised level. 63M × 80% = 50.4M.
    advance_to_next_day(&mut suite);

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
        .should_succeed();
}

#[test]
fn personal_quota() {
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
        .should_succeed();

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(PersonalQuota {
            amount: Uint128::new(50_000_000),
            expiry: None,
        }));

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
        .should_succeed();

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(PersonalQuota {
            amount: Uint128::new(10_000_000),
            expiry: None,
        }));

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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

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
        .should_succeed();

    // 2h later — under the 24h cron interval, so the global quota stays at 0.
    advance_by(&mut suite, Duration::from_hours(2));

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
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, requested: 1, remaining after personal quota: 1");

    // The expired entry is left in storage; the handler doesn't scrub it. The
    // caller can still query it to reason about `expiry`.
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
    assert!(stored.and_then(|q| q.expiry).is_some());

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
#[test]
fn personal_quota_revoke_via_op_delete() {
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

    // 100M supply. 10% rate limit → global quota = 10M.
    suite
        .receive_warp_transfer(
            relayer,
            mock_solana::DOMAIN,
            mock_solana::USDC_WARP,
            receiver,
            100_000_000,
        )
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
        .should_succeed();

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(PersonalQuota {
            amount: Uint128::new(50_000_000),
            expiry: None,
        }));

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
        .should_fail_with_error(
            "insufficient outbound quota! denom: bridge/usdc, requested: 20000000, remaining after personal quota: 20000000",
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
        .should_succeed();
}

/// Granting a personal quota for a denom that is NOT rate-limited globally
/// must still behave correctly: the personal allowance is consumed first,
/// and any overflow falls through to an absent global entry (which means
/// unrestricted, not "blocked").
#[test]
fn personal_quota_on_un_rate_limited_denom() {
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
        .should_succeed();

    suite
        .query_wasm_smart(contracts.gateway, gateway::QueryPersonalQuotaRequest {
            user: receiver_addr,
            denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(Some(PersonalQuota {
            amount: Uint128::new(20_000_000),
            expiry: None,
        }));

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
        .should_succeed();
}

fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}

fn advance_by(suite: &mut TestSuite, d: Duration) {
    suite.block_time = d;
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}
