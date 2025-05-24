use {
    dango_testing::{HyperlaneTestSuite, TestOption, TestSuite, setup_test},
    dango_types::{
        constants::usdc,
        gateway::{self, RateLimit, Remote, WarpRemote},
    },
    grug::{Addr, BalanceChange, Coin, Coins, Duration, QuerierExt, ResultExt, Udec128, btree_map},
    hyperlane_types::{
        Addr32,
        constants::{ethereum, solana},
    },
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
            (ethereum::DOMAIN, ethereum::USDC_WARP, 100_000_000),
            (solana::DOMAIN, solana::USDC_WARP, 200_000_000),
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
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 30_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Trigger the rate limit sending 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, amount: 1");

    // Receive more tokens increase rate limit and allow to send them back.
    suite
        .receive_warp_transfer(
            relayer,
            ethereum::DOMAIN,
            ethereum::USDC_WARP,
            receiver,
            100_000_000,
        )
        .should_succeed();

    // Check supply now.
    // it should be 200_000_000 + 200_000_000 - 30_000_000 + 100_000_000 = 370_000_000
    // `receiver` should has 370_000_000 - 10_000 (fee) = 369_990_000
    {
        suite
            .query_supply(usdc::DENOM.clone())
            .should_succeed_and_equal(370_000_000.into());

        suite.balances().should_change(receiver, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Increased(369_990_000),
        });
    }

    // Try withdraw everything the 100_000 available but to ethereum.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::USDC_WARP,
                },
                recipient: mock_eth_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 100_000_000 + usdc_eth_fee).unwrap(),
        )
        .should_succeed();

    // Trigger the rate limit sending 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::USDC_WARP,
                },
                recipient: mock_eth_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 1 + usdc_eth_fee).unwrap(),
        )
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, amount: 1");

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // 370_000_000 - 100_000_000 = 270_000_000 where
    // 100_000_000 are from ethereum
    // 170_000_000 are from solana

    // Check reserves.
    for (remote, amount) in [
        (
            Remote::Warp(WarpRemote {
                domain: ethereum::DOMAIN,
                contract: ethereum::USDC_WARP,
            }),
            100_000_000,
        ),
        (
            Remote::Warp(WarpRemote {
                domain: solana::DOMAIN,
                contract: solana::USDC_WARP,
            }),
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

    // Withdraw 27 tokens to solana.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 27_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // Try to withdraw 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient outbound quota! denom: bridge/usdc, amount: 1");

    // Increase the rate limit
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

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // 370_000_000 - 100_000_000 - 27_000_000 = 243_000_000 where
    // 100_000_000 are from ethereum
    // 143_000_000 are from solana

    // try to withdraw 43 to solana.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 143_000_000 + usdc_sol_fee).unwrap(),
        )
        .should_succeed();

    // solana should be empty now.
    // Try to withdraw 1 more token.
    suite
        .execute(
            receiver,
            contracts.gateway,
            &gateway::ExecuteMsg::TransferRemote(gateway::bridge::TransferRemoteRequest::Warp {
                warp_remote: WarpRemote {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                },
                recipient: mock_solana_recipient,
            }),
            Coin::new(usdc::DENOM.clone(), 1 + usdc_sol_fee).unwrap(),
        )
        .should_fail_with_error("insufficient reserve!");
}

fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}
