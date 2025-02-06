use {
    assertor::*,
    dango_testing::{
        setup_test, setup_test_with_indexer, HyperlaneTestSuite, MOCK_LOCAL_DOMAIN,
        MOCK_REMOTE_DOMAIN,
    },
    dango_types::{
        constants::{DANGO_DENOM, ETH_DENOM, SOL_DENOM},
        warp::{self, Route, TokenMessage},
    },
    dango_warp::ROUTES,
    grug::{
        btree_map, Addr, Addressable, BalanceChange, Coin, Coins, Denom, HashExt, HexBinary,
        MathError, NumberConst, QuerierExt, ResultExt, StdError, Uint128,
    },
    hyperlane_types::{
        addr32,
        hooks::merkle,
        mailbox::{self, Message, MAILBOX_VERSION},
        Addr32, IncrementalMerkleTree,
    },
    sea_orm::EntityTrait,
    std::{ops::DerefMut, str::FromStr},
};

const MOCK_ROUTE: Route = Route {
    address: addr32!("0000000000000000000000000000000000000000000000000000000000000000"),
    fee: Uint128::new(25),
};

const MOCK_RECIPIENT: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000001");

#[test]
fn send_escrowing_collateral() {
    let ((mut suite, mut accounts, _, contracts), _) = setup_test_with_indexer();

    let metadata = HexBinary::from_inner(b"hello".to_vec());

    // Attempt to send before a route is set.
    // Should fail with route not found error.
    suite
        .execute(
            &mut accounts.user1,
            contracts.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(DANGO_DENOM.clone(), 100).unwrap(),
        )
        .should_fail_with_error(StdError::data_not_found::<Route>(
            ROUTES
                .path((&DANGO_DENOM, MOCK_REMOTE_DOMAIN))
                .storage_key(),
        ));

    // Owner sets the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: DANGO_DENOM.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query the route. Should have been set.
    suite
        .query_wasm_smart(contracts.warp, warp::QueryRouteRequest {
            denom: DANGO_DENOM.clone(),
            destination_domain: MOCK_REMOTE_DOMAIN,
        })
        .should_succeed_and_equal(MOCK_ROUTE);

    // Try sending again, should work.
    suite
        .execute(
            &mut accounts.user1,
            contracts.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(DANGO_DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    // The message should have been inserted into Merkle tree.
    suite
        .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
        .should_succeed_and_equal({
            let token_msg = TokenMessage {
                recipient: MOCK_RECIPIENT,
                amount: Uint128::new(100) - MOCK_ROUTE.fee,
                metadata,
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: MOCK_LOCAL_DOMAIN,
                sender: contracts.warp.into(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_ROUTE.address,
                body: token_msg.encode(),
            };

            let mut tree = IncrementalMerkleTree::default();
            tree.insert(msg.encode().keccak256()).unwrap();
            tree
        });

    // The fee hook should have received the fee.
    suite
        .query_balance(&contracts.hyperlane.fee, DANGO_DENOM.clone())
        .should_succeed_and_equal(MOCK_ROUTE.fee);

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // The transfers should have been indexed.
    suite.app.indexer.handle.block_on(async {
        let blocks = indexer_sql::entity::blocks::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");

        assert_that!(blocks).has_length(3);

        let transfers = dango_indexer_sql::entity::transfers::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch transfers");

        assert_that!(transfers).has_length(3);

        assert_that!(transfers
            .iter()
            .map(|t| t.amount.as_str())
            .collect::<Vec<_>>())
        .is_equal_to(vec!["100", "25", "25"]);
    });
}

#[test]
fn send_burning_synth() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let metadata = HexBinary::from_inner(b"foo".to_vec());

    // Set the route for the synth token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: ETH_DENOM.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Send the tokens.
    suite
        .execute(
            &mut accounts.user1,
            contracts.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(ETH_DENOM.clone(), 12345).unwrap(),
        )
        .should_succeed();

    // Message should have been inserted into the Merkle tree.
    suite
        .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
        .should_succeed_and_equal({
            let token_msg = TokenMessage {
                recipient: MOCK_RECIPIENT,
                amount: Uint128::new(12345) - MOCK_ROUTE.fee,
                metadata,
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: MOCK_LOCAL_DOMAIN,
                sender: contracts.warp.into(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_ROUTE.address,
                body: token_msg.encode(),
            };

            let mut tree = IncrementalMerkleTree::default();
            tree.insert(msg.encode().keccak256()).unwrap();
            tree
        });

    // Sender should have been deducted balance.
    suite
        .query_balance(&accounts.user1, ETH_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 12345));

    // Warp contract should not hold any of the synth token (should be burned).
    suite
        .query_balance(&contracts.warp, ETH_DENOM.clone())
        .should_succeed_and_equal(Uint128::ZERO);

    // Fee hook should have received the fee.
    suite
        .query_balance(&contracts.hyperlane.fee, ETH_DENOM.clone())
        .should_succeed_and_equal(MOCK_ROUTE.fee);
}

#[test]
fn receive_release_collateral() {
    let (suite, mut accounts, _, contracts) = setup_test();
    let (mut suite, ..) = HyperlaneTestSuite::new(
        suite,
        accounts.owner,
        btree_map! { MOCK_REMOTE_DOMAIN => (3, 2) },
    );

    // Set the route.
    suite
        .hyperlane()
        .set_route(DANGO_DENOM.clone(), MOCK_REMOTE_DOMAIN, MOCK_ROUTE)
        .should_succeed();

    // Send some tokens so that we have something to release.
    suite
        .hyperlane()
        .send_transfer(
            &mut accounts.user1,
            MOCK_REMOTE_DOMAIN,
            MOCK_RECIPIENT,
            Coin::new(DANGO_DENOM.clone(), 125).unwrap(),
        )
        .should_succeed();

    // Now, receive a message from the origin domain.
    let message_id = suite
        .hyperlane()
        .receive_transfer(
            MOCK_REMOTE_DOMAIN,
            accounts.user1.address(),
            Coin::new(DANGO_DENOM.clone(), 88).unwrap(),
        )
        .message_id;

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest { message_id },
        )
        .should_succeed_and_equal(true);

    // The recipient should have received the tokens.
    suite
        .query_balance(&accounts.user1, DANGO_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 125 + 88));

    // Warp contract should have been deducted tokens.
    suite
        .query_balance(&contracts.warp, DANGO_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(100 - 88));
}

#[test]
fn receive_minting_synth() {
    let (suite, accounts, _, contracts) = setup_test();
    let (mut suite, ..) = HyperlaneTestSuite::new(
        suite,
        accounts.owner,
        btree_map! {MOCK_REMOTE_DOMAIN => (3, 2)},
    );

    // Set the route.
    suite
        .hyperlane()
        .set_route(SOL_DENOM.clone(), MOCK_REMOTE_DOMAIN, MOCK_ROUTE)
        .should_succeed();

    // Now, receive a message from the origin domain.
    let message_id = suite
        .hyperlane()
        .receive_transfer(
            MOCK_REMOTE_DOMAIN,
            accounts.user1.address(),
            Coin::new(SOL_DENOM.clone(), 88).unwrap(),
        )
        .message_id;

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest { message_id },
        )
        .should_succeed_and_equal(true);

    // Synthetic tokens should have been minted to the receiver.
    suite
        .query_balance(&accounts.user1, SOL_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(88));
}

#[test]
fn alloy() {
    let (suite, mut accounts, _, contracts) = setup_test();

    let eth_domain = 10;
    let sol_domain = 20;

    let sol_usdc_denom = Denom::from_str("hyp/sol/usdc").unwrap();
    let eth_usdc_denom = Denom::from_str("hyp/eth/usdc").unwrap();

    let eth_usdc_recipient = Addr::mock(1).into();
    let sol_usdc_recipient = Addr::mock(2).into();

    let alloyed_usdc_denom = Denom::from_str("hyp/all/usdc").unwrap();

    let (mut suite, owner) = HyperlaneTestSuite::new(suite, accounts.owner, btree_map! {
        eth_domain => (3, 2),
        sol_domain => (3, 2),
    });

    // Set the route.
    for (domain, denom, route) in [
        (eth_domain, &eth_usdc_denom, Route {
            address: eth_usdc_recipient,
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

    // Receive some tokens.
    {
        suite
            .balances()
            .record_many([accounts.user1.address(), contracts.warp.address()]);

        suite.hyperlane().receive_transfer(
            eth_domain,
            accounts.user1.address(),
            Coin::new(eth_usdc_denom.clone(), 100).unwrap(),
        );

        suite.hyperlane().receive_transfer(
            sol_domain,
            accounts.user1.address(),
            Coin::new(sol_usdc_denom.clone(), 200).unwrap(),
        );

        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Increased(100),
                sol_usdc_denom.clone() => BalanceChange::Increased(200),
            });
    }

    // Register Alloy.
    for (domain, denom) in [(eth_domain, &eth_usdc_denom), (sol_domain, &sol_usdc_denom)] {
        suite
            .execute(
                owner.write_access().deref_mut(),
                contracts.warp,
                &warp::ExecuteMsg::SetAlloy {
                    underlying_denom: denom.clone(),
                    destination_domain: domain,
                    alloyed_denom: alloyed_usdc_denom.clone(),
                },
                Coins::new(),
            )
            .should_succeed();
    }

    // Receive more tokens. Now they should be alloyed.
    {
        suite.hyperlane().receive_transfer(
            eth_domain,
            accounts.user1.address(),
            Coin::new(eth_usdc_denom.clone(), 50).unwrap(),
        );

        suite.hyperlane().receive_transfer(
            sol_domain,
            accounts.user1.address(),
            Coin::new(sol_usdc_denom.clone(), 75).unwrap(),
        );

        // Verify balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Increased(100),
                sol_usdc_denom.clone() => BalanceChange::Increased(200),
                alloyed_usdc_denom.clone() => BalanceChange::Increased(125),
            });

        suite
            .balances()
            .should_change(contracts.warp.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Increased(50),
                sol_usdc_denom.clone() => BalanceChange::Increased(75),
            });
    }

    // Recap the balances of user1.
    // eth_usdc => 100
    // sol_usdc => 200
    // alloyed_usdc => 125 | 50 from eth, 75 from sol

    // Send 20 alloyed_usdc to eth.
    {
        suite.balances().refresh_all();

        // Get the current merkle tree.
        let mut tree = suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed();

        let recipient: Addr32 = Addr::mock(2).into();

        // Send 20 alloyed to eth.
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                eth_domain,
                recipient,
                Coin::new(alloyed_usdc_denom.clone(), 20).unwrap(),
            )
            .should_succeed();

        // Insert the message into the tree.
        let token_msg = TokenMessage {
            recipient,
            amount: Uint128::new(20),
            metadata: Default::default(),
        };

        let msg = Message {
            version: MAILBOX_VERSION,
            nonce: 0,
            origin_domain: MOCK_LOCAL_DOMAIN,
            sender: contracts.warp.into(),
            destination_domain: eth_domain,
            recipient: eth_usdc_recipient,
            body: token_msg.encode(),
        };

        tree.insert(msg.encode().keccak256()).unwrap();

        // Check if the merkle tree has been updated.
        suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed_and_equal(tree);

        // Verify balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                alloyed_usdc_denom.clone() => BalanceChange::Decreased(20),
            });

        suite
            .balances()
            .should_change(contracts.warp.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Decreased(20),
            });
    }

    // 20 alloyed_usdc has been sent via eth.
    // Try sent 35 more. This should fail (30 left).
    {
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                eth_domain,
                Addr::mock(2).into(),
                Coin::new(alloyed_usdc_denom.clone(), 35).unwrap(),
            )
            .should_fail_with_error(MathError::overflow_sub::<u128>(30, 35));
    }

    // Send 75 alloyed_usdc to sol.
    {
        suite.balances().refresh_all();

        let recipient: Addr32 = Addr::mock(2).into();

        // Get the current merkle tree.
        let mut tree = suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed();

        // Send all sol_usdc.
        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                sol_domain,
                recipient,
                Coin::new(alloyed_usdc_denom.clone(), 75).unwrap(),
            )
            .should_succeed();

        // Insert the message into the tree.
        let token_msg = TokenMessage {
            recipient,
            amount: Uint128::new(75),
            metadata: Default::default(),
        };

        let msg = Message {
            version: MAILBOX_VERSION,
            nonce: 1,
            origin_domain: MOCK_LOCAL_DOMAIN,
            sender: contracts.warp.into(),
            destination_domain: sol_domain,
            recipient: sol_usdc_recipient,
            body: token_msg.encode(),
        };

        tree.insert(msg.encode().keccak256()).unwrap();

        // Check if the merkle tree has been updated.
        suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed_and_equal(tree);

        // Verify balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                alloyed_usdc_denom.clone() => BalanceChange::Decreased(75),
            });

        suite
            .balances()
            .should_change(contracts.warp.address(), btree_map! {
                sol_usdc_denom.clone() => BalanceChange::Decreased(75),
            });
    }

    // Send 100 eth_usdc to eth.
    {
        suite.balances().refresh_all();

        let recipient: Addr32 = Addr::mock(2).into();

        // Get the current merkle tree.
        let mut tree = suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed();

        suite
            .hyperlane()
            .send_transfer(
                &mut accounts.user1,
                eth_domain,
                Addr::mock(2).into(),
                Coin::new(eth_usdc_denom.clone(), 100).unwrap(),
            )
            .should_succeed();

        // Insert the message into the tree.
        let token_msg = TokenMessage {
            recipient,
            amount: Uint128::new(100),
            metadata: Default::default(),
        };

        let msg = Message {
            version: MAILBOX_VERSION,
            nonce: 2,
            origin_domain: MOCK_LOCAL_DOMAIN,
            sender: contracts.warp.into(),
            destination_domain: eth_domain,
            recipient: eth_usdc_recipient,
            body: token_msg.encode(),
        };

        tree.insert(msg.encode().keccak256()).unwrap();

        // Check if the merkle tree has been updated.
        suite
            .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
            .should_succeed_and_equal(tree);

        // Verify balances.
        suite
            .balances()
            .should_change(accounts.user1.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Decreased(100),
            });

        // No changes on warp balances.
        suite
            .balances()
            .should_change(contracts.warp.address(), btree_map! {
                eth_usdc_denom.clone() => BalanceChange::Unchanged,
            });
    }
}
