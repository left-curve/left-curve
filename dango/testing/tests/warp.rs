use {
    dango_testing::setup_test,
    grug::{
        Addressable, Coins, Denom, HashExt, HexBinary, NumberConst, ResultExt, StdError, Uint128,
    },
    hyperlane_types::{
        addr32,
        mailbox::{self, Message, MAILBOX_VERSION},
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
fn receive_release_collateral() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let denom = Denom::from_str("udng").unwrap();
    let origin_domain = 123;

    // Set the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain: origin_domain,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Send some tokens so that we have something to release.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: origin_domain,
                recipient: MOCK_RECIPIENT,
                metadata: None,
            },
            Coins::one(denom.clone(), 100).unwrap(),
        )
        .should_succeed();

    // Now, receive a message from the origin domain.
    let raw_message = Message {
        version: MAILBOX_VERSION,
        nonce: 0,
        origin_domain,
        sender: MOCK_ROUTE,
        destination_domain: 12345678, // this should be our local domain
        recipient: contracts.hyperlane.warp.into(),
        body: TokenMessage {
            recipient: accounts.relayer.address().into(),
            amount: Uint128::new(88),
            metadata: HexBinary::default(),
        }
        .encode(),
    }
    .encode();

    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.mailbox,
            &mailbox::ExecuteMsg::Process {
                raw_message: raw_message.clone(),
                metadata: HexBinary::default(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest {
                message_id: raw_message.keccak256(),
            },
        )
        .should_succeed_and_equal(true);

    // The recipient should have received the tokens.
    suite
        .query_balance(&accounts.relayer, denom.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 100 + 88));

    // Warp contract should have been deducted tokens.
    suite
        .query_balance(&contracts.hyperlane.warp, denom)
        .should_succeed_and_equal(Uint128::new(100 - 88));
}

#[test]
fn receive_minting_synth() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let denom = Denom::from_str("hpl/solana/fartcoin").unwrap();
    let origin_domain = 123;

    // Set the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain: origin_domain,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Now, receive a message from the origin domain.
    let raw_message = Message {
        version: MAILBOX_VERSION,
        nonce: 0,
        origin_domain,
        sender: MOCK_ROUTE,
        destination_domain: 12345678, // this should be our local domain
        recipient: contracts.hyperlane.warp.into(),
        body: TokenMessage {
            recipient: accounts.relayer.address().into(),
            amount: Uint128::new(88),
            metadata: HexBinary::default(),
        }
        .encode(),
    }
    .encode();

    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.mailbox,
            &mailbox::ExecuteMsg::Process {
                raw_message: raw_message.clone(),
                metadata: HexBinary::default(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest {
                message_id: raw_message.keccak256(),
            },
        )
        .should_succeed_and_equal(true);

    // Synthetic tokens should have been minted to the receiver.
    suite
        .query_balance(&accounts.relayer, denom.clone())
        .should_succeed_and_equal(Uint128::new(88));
}