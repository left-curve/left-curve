use {
    dango_testing::setup_test,
    grug::{Coins, Denom, HashExt, HexBinary, NumberConst, ResultExt, StdError, Uint128},
    hyperlane_mailbox::MAILBOX_VERSION,
    hyperlane_types::{
        addr32,
        mailbox::Message,
        merkle,
        merkle_tree::MerkleTree,
        warp::{self, TokenMessage},
        Addr32,
    },
    hyperlane_warp::ROUTES,
    std::str::FromStr,
};

pub const MOCK_RECIPIENT: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000000");

pub const MOCK_ROUTE: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000001");

#[test]
fn send_escrowing_collateral() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let denom = Denom::from_str("udng").unwrap();
    let destination_domain = 123;
    let metadata = HexBinary::from_inner(b"hello".to_vec());

    // Attempt to send before a route is set.
    // Should fail with route not found error.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one("udng", 100).unwrap(),
        )
        .should_fail_with_error(StdError::data_not_found::<Addr32>(
            ROUTES.path((&denom, 123)).storage_key(),
        ));

    // Owner sets the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query the route. Should have been set.
    suite
        .query_wasm_smart(contracts.hyperlane.warp, warp::QueryRouteRequest {
            denom: denom.clone(),
            destination_domain,
        })
        .should_succeed_and_equal(MOCK_ROUTE);

    // Try sending again, should work.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(denom, 100).unwrap(),
        )
        .should_succeed();

    // The message should have been inserted into Merkle tree.
    suite
        .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
        .should_succeed_and_equal({
            let token_msg = TokenMessage {
                recipient: MOCK_RECIPIENT,
                amount: Uint128::new(100),
                metadata,
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: 12345678,
                sender: contracts.hyperlane.warp.into(),
                destination_domain,
                recipient: MOCK_ROUTE,
                body: token_msg.encode(),
            };

            let mut tree = MerkleTree::default();
            tree.insert(msg.encode().keccak256()).unwrap();
            tree
        });
}

#[test]
fn send_burning_synth() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let denom = Denom::from_str("hpl/ethereum/ether").unwrap();
    let destination_domain = 123;
    let metadata = HexBinary::from_inner(b"foo".to_vec());

    // Set the route for the synth token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Send the tokens.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(denom.clone(), 12345).unwrap(),
        )
        .should_succeed();

    // Message should have been inserted into the Merkle tree.
    suite
        .query_wasm_smart(contracts.hyperlane.merkle, merkle::QueryTreeRequest {})
        .should_succeed_and_equal({
            let token_msg = TokenMessage {
                recipient: MOCK_RECIPIENT,
                amount: Uint128::new(12345),
                metadata,
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: 12345678,
                sender: contracts.hyperlane.warp.into(),
                destination_domain,
                recipient: MOCK_ROUTE,
                body: token_msg.encode(),
            };

            let mut tree = MerkleTree::default();
            tree.insert(msg.encode().keccak256()).unwrap();
            tree
        });

    // Sender should have been deducted balance.
    suite
        .query_balance(&accounts.relayer, denom.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 12345));

    // Warp contract should not hold any of the synth token (should be burned).
    suite
        .query_balance(&contracts.hyperlane.warp, denom)
        .should_succeed_and_equal(Uint128::ZERO);
}

#[test]
fn receive_release_collateral() {}

#[test]
fn receive_minting_synth() {}
