use {
    dango_testing::{HyperlaneTestSuite, TestSuite, setup_test},
    dango_types::warp::{self},
    grug::{
        Addr, Addressable, BalanceChange, Coin, Coins, Denom, Duration, NumberConst, ResultExt,
        Udec128, Uint128, btree_map,
    },
    std::str::FromStr,
};

#[test]
fn rate_limit() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test(Default::default());

    suite.block_time = Duration::ZERO;

    let osmo_domain = 10;
    let sol_domain = 20;

    let osmo_usdc_recipient = Addr::mock(1).into();
    let sol_usdc_recipient = Addr::mock(2).into();
    let mock_remote_user = Addr::mock(3).into();

    let sol_usdc_denom = Denom::from_str("hyp/sol/usdc").unwrap();
    let osmo_usdc_denom = Denom::from_str("hyp/osmo/usdc").unwrap();

    let alloyed_usdc_denom = Denom::from_str("hyp/all/usdc").unwrap();

    let (mut suite, owner) = HyperlaneTestSuite::new(suite, accounts.owner, btree_map! {
        osmo_domain => (3, 2),
        sol_domain => (3, 2),
    });

    // Set the route.
    for (domain, denom, route) in [
        (osmo_domain, &osmo_usdc_denom, Route {
            address: osmo_usdc_recipient,
            fee: Uint128::ZERO,
        }),
        (sol_domain, &sol_usdc_denom, Route {
            address: sol_usdc_recipient,
            fee: Uint128::ZERO,
        }),
    ] {
        suite
            .hyperlane()
            .set_route(denom.clone(), domain, route)
            .should_succeed();
    }

    suite.balances().record(accounts.user1.address());

    // Receive some tokens.
    // osmo_usdc => 100
    // sol_usdc => 200
    {
        for (domain, denom, amount) in [
            (osmo_domain, &osmo_usdc_denom, 100),
            (sol_domain, &sol_usdc_denom, 200),
        ] {
            suite.hyperlane().receive_transfer(
                domain,
                accounts.user1.address(),
                coin(denom, amount),
            );
        }

        // Check balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                osmo_usdc_denom.clone() => BalanceChange::Increased(100),
                sol_usdc_denom.clone() => BalanceChange::Increased(200),
            });
    }

    // Set rate limit.
    // osmo_usdc => 10%
    // sol_usdc => 20%
    suite
        .execute(
            owner.write_access().deref_mut(),
            contracts.warp,
            &warp::ExecuteMsg::SetRateLimits(btree_map! {
                osmo_usdc_denom.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
                sol_usdc_denom.clone()  => RateLimit::new_unchecked(Udec128::new_percent(20)),
            }),
            Coins::default(),
        )
        .should_succeed();

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // Try send back exact tokens to don't trigger rate limit.
    // osmo_usdc => 100 * 0.1 = 10
    // sol_usdc => 200 * 0.2 = 40
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 10),
        (sol_domain, &sol_usdc_denom, 40),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Trigger the rate limit sending 1 more token.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Receive more tokens increase rate limit and allow to send them back.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 100),
        (sol_domain, &sol_usdc_denom, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));

        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // osmo_usdc => 100 - 10 = 90
    // sol_usdc => 200 - 40 = 160

    // New limits are:
    // osmo_usdc => 90 * 0.1 = 9
    // sol_usdc => 160 * 0.2 = 32

    // Try send them back.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 9),
        (sol_domain, &sol_usdc_denom, 32),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Trigger the rate limit sending 1 more token.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Receive some tokens to reset back the supply to:
    // osmo_usdc => 100
    // sol_usdc => 200
    for (domain, denom, amount, should_be) in [
        (osmo_domain, &osmo_usdc_denom, 10 + 9, 100),
        (sol_domain, &sol_usdc_denom, 40 + 32, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));

        suite
            .query_supply(denom.clone())
            .should_succeed_and_equal(Uint128::new(should_be));
    }

    // Create alloy token
    for (domain, denom) in [
        (osmo_domain, &osmo_usdc_denom),
        (sol_domain, &sol_usdc_denom),
    ] {
        suite
            .execute(
                owner.write_access().deref_mut(),
                contracts.warp,
                &warp::ExecuteMsg::SetAlloy {
                    underlying_denom: denom.clone(),
                    destination_domain: domain,
                    alloyed_denom: alloyed_usdc_denom.clone(),
                },
                Coins::default(),
            )
            .should_succeed();
    }

    // Receive some tokens. they should be minted as alloyed:
    // osmo_usdc => 100
    // sol_usdc  => 200
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 100),
        (sol_domain, &sol_usdc_denom, 200),
    ] {
        suite
            .hyperlane()
            .receive_transfer(domain, accounts.user1.address(), coin(denom, amount));
    }

    // Make 1 day pass letting the cron job to reset the rate limits.
    advance_to_next_day(&mut suite);

    // The supply on chain now are:
    // osmo_usdc => 100 + 100 = 200
    // sol_usdc  => 200 + 200 = 400

    // Send the alloyed tokens.
    for (domain, denom, amount) in [
        (osmo_domain, &alloyed_usdc_denom, 20),
        (sol_domain, &alloyed_usdc_denom, 80),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_succeed();
    }

    // Send 1 more tokens to trigger the rate limit.
    for (domain, denom, amount) in [
        (osmo_domain, &alloyed_usdc_denom, 1),
        (sol_domain, &alloyed_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }

    // Try send 1 of the underlying tokens. Rate limit should be triggered.
    for (domain, denom, amount) in [
        (osmo_domain, &osmo_usdc_denom, 1),
        (sol_domain, &sol_usdc_denom, 1),
    ] {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                domain,
                mock_remote_user,
                coin(denom, amount),
            )
            .should_fail_with_error("rate limit reached: 0 < 1");
    }
}

fn advance_to_next_day(suite: &mut TestSuite) {
    suite.block_time = Duration::from_days(1);
    suite.make_empty_block();
    suite.block_time = Duration::ZERO;
}

fn coin(denom: &Denom, amount: u128) -> Coin {
    Coin::new(denom.clone(), amount).unwrap()
}
