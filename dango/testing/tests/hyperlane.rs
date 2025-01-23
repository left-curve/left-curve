use {
    assertor::*,
    dango_testing::{generate_random_key, setup_test, setup_test_with_indexer},
    grug::{
        Addressable, Coins, Denom, Hash256, HashExt, HexBinary, HexByteArray, Inner, NumberConst,
        QuerierExt, ResultExt, StdError, Uint128,
    },
    grug_crypto::Identity256,
    hyperlane_types::{
        addr32, domain_hash, eip191_hash,
        hooks::merkle,
        isms::{self, multisig::Metadata, HYPERLANE_DOMAIN_KEY},
        mailbox::{self, Domain, Message, MAILBOX_VERSION},
        multisig_hash,
        recipients::warp::{self, Route, TokenMessage},
        Addr32, IncrementalMerkleTree,
    },
    hyperlane_warp::ROUTES,
    k256::ecdsa::SigningKey,
    sea_orm::EntityTrait,
    std::{collections::BTreeSet, str::FromStr},
};

const MOCK_ROUTE: Route = Route {
    address: addr32!("0000000000000000000000000000000000000000000000000000000000000000"),
    fee: Uint128::new(25),
};

const MOCK_RECIPIENT: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000001");

const MOCK_REMOTE_MERKLE_TREE: Addr32 =
    addr32!("0000000000000000000000000000000000000000000000000000000000000002");

const MOCK_REMOTE_DOMAIN: Domain = 123;

const MOCK_LOCAL_DOMAIN: Domain = 88888888;

struct MockValidatorSet {
    secrets: Vec<SigningKey>,
    addresses: BTreeSet<HexByteArray<20>>,
    merkle_tree: IncrementalMerkleTree,
}

impl MockValidatorSet {
    pub fn new_random(size: usize) -> Self {
        let (secrets, addresses) = (0..size)
            .map(|_| {
                let (sk, _) = generate_random_key();
                // We need the _uncompressed_ pubkey for deriving Ethereum address.
                let pk = sk.verifying_key().to_encoded_point(false).to_bytes();
                let pk_hash = (&pk[1..]).keccak256();
                let address = &pk_hash[12..];

                (sk, HexByteArray::from_inner(address.try_into().unwrap()))
            })
            .unzip();

        Self {
            secrets,
            addresses,
            merkle_tree: IncrementalMerkleTree::default(),
        }
    }

    pub fn sign(&mut self, message_id: Hash256) -> Metadata {
        self.merkle_tree.insert(message_id).unwrap();

        let merkle_root = self.merkle_tree.root();
        let merkle_index = (self.merkle_tree.count - 1) as u32;

        let multisig_hash = eip191_hash(multisig_hash(
            domain_hash(
                MOCK_REMOTE_DOMAIN,
                MOCK_REMOTE_MERKLE_TREE,
                HYPERLANE_DOMAIN_KEY,
            ),
            merkle_root,
            merkle_index,
            message_id,
        ));

        let signatures = self
            .secrets
            .iter()
            .map(|sk| {
                let (signature, recovery_id) = sk
                    .sign_digest_recoverable(Identity256::from(multisig_hash.into_inner()))
                    .unwrap();

                let mut packed = [0u8; 65];
                packed[..64].copy_from_slice(&signature.to_bytes());
                packed[64] = recovery_id.to_byte() + 27;
                HexByteArray::from_inner(packed)
            })
            .collect();

        Metadata {
            origin_merkle_tree: MOCK_REMOTE_MERKLE_TREE,
            merkle_root,
            merkle_index,
            signatures,
        }
    }
}

#[test]
fn send_escrowing_collateral() {
    let (mut suite, mut accounts, _, contracts) = setup_test_with_indexer();

    let denom = Denom::from_str("udng").unwrap();
    let metadata = HexBinary::from_inner(b"hello".to_vec());

    // Attempt to send before a route is set.
    // Should fail with route not found error.
    suite
        .execute(
            &mut accounts.user1,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one("udng", 100).unwrap(),
        )
        .should_fail_with_error(StdError::data_not_found::<Route>(
            ROUTES.path((&denom, MOCK_REMOTE_DOMAIN)).storage_key(),
        ));

    // Owner sets the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query the route. Should have been set.
    suite
        .query_wasm_smart(contracts.hyperlane.warp, warp::QueryRouteRequest {
            denom: denom.clone(),
            destination_domain: MOCK_REMOTE_DOMAIN,
        })
        .should_succeed_and_equal(MOCK_ROUTE);

    // Try sending again, should work.
    suite
        .execute(
            &mut accounts.user1,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: Some(metadata.clone()),
            },
            Coins::one(denom.clone(), 100).unwrap(),
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
                sender: contracts.hyperlane.warp.into(),
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
        .query_balance(&contracts.hyperlane.fee, denom)
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

    let denom = Denom::from_str("hyp/ethereum/ether").unwrap();
    let metadata = HexBinary::from_inner(b"foo".to_vec());

    // Set the route for the synth token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
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
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
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
                amount: Uint128::new(12345) - MOCK_ROUTE.fee,
                metadata,
            };
            let msg = Message {
                version: MAILBOX_VERSION,
                nonce: 0,
                origin_domain: MOCK_LOCAL_DOMAIN,
                sender: contracts.hyperlane.warp.into(),
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
        .query_balance(&accounts.user1, denom.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 12345));

    // Warp contract should not hold any of the synth token (should be burned).
    suite
        .query_balance(&contracts.hyperlane.warp, denom.clone())
        .should_succeed_and_equal(Uint128::ZERO);

    // Fee hook should have received the fee.
    suite
        .query_balance(&contracts.hyperlane.fee, denom)
        .should_succeed_and_equal(MOCK_ROUTE.fee);
}

#[test]
fn receive_release_collateral() {
    let (mut suite, mut accounts, _, contracts) = setup_test();
    let mut mock_validator_set = MockValidatorSet::new_random(3);

    let denom = Denom::from_str("udng").unwrap();

    // Set validators at the ISM.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.ism,
            &isms::multisig::ExecuteMsg::SetValidators {
                domain: MOCK_REMOTE_DOMAIN,
                threshold: 2,
                validators: mock_validator_set.addresses.clone(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Set the route.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::SetRoute {
                denom: denom.clone(),
                destination_domain: MOCK_REMOTE_DOMAIN,
                route: MOCK_ROUTE,
            },
            Coins::new(),
        )
        .should_succeed();

    // Send some tokens so that we have something to release.
    suite
        .execute(
            &mut accounts.user1,
            contracts.hyperlane.warp,
            &warp::ExecuteMsg::TransferRemote {
                destination_domain: MOCK_REMOTE_DOMAIN,
                recipient: MOCK_RECIPIENT,
                metadata: None,
            },
            Coins::one(denom.clone(), 125).unwrap(),
        )
        .should_succeed();

    // Now, receive a message from the origin domain.
    let raw_message = Message {
        version: MAILBOX_VERSION,
        nonce: 0,
        origin_domain: MOCK_REMOTE_DOMAIN,
        sender: MOCK_ROUTE.address,
        destination_domain: MOCK_LOCAL_DOMAIN, // this should be our local domain
        recipient: contracts.hyperlane.warp.into(),
        body: TokenMessage {
            recipient: accounts.user1.address().into(),
            amount: Uint128::new(88),
            metadata: HexBinary::default(),
        }
        .encode(),
    }
    .encode();

    let message_id = raw_message.keccak256();
    let raw_metadata = mock_validator_set.sign(message_id).encode();

    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.mailbox,
            &mailbox::ExecuteMsg::Process {
                raw_message: raw_message.clone(),
                raw_metadata,
            },
            Coins::new(),
        )
        .should_succeed();

    // The message should have been recorded as received.
    suite
        .query_wasm_smart(
            contracts.hyperlane.mailbox,
            mailbox::QueryDeliveredRequest { message_id },
        )
        .should_succeed_and_equal(true);

    // The recipient should have received the tokens.
    suite
        .query_balance(&accounts.user1, denom.clone())
        .should_succeed_and_equal(Uint128::new(100_000_000_000_000 - 125 + 88));

    // Warp contract should have been deducted tokens.
    suite
        .query_balance(&contracts.hyperlane.warp, denom)
        .should_succeed_and_equal(Uint128::new(100 - 88));
}

#[test]
fn receive_minting_synth() {
    let (mut suite, mut accounts, _, contracts) = setup_test();
    let mut mock_validator_set = MockValidatorSet::new_random(3);

    let denom = Denom::from_str("hyp/solana/fartcoin").unwrap();
    let origin_domain = 123;

    // Set validators at the ISM.
    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.ism,
            &isms::multisig::ExecuteMsg::SetValidators {
                domain: MOCK_REMOTE_DOMAIN,
                threshold: 2,
                validators: mock_validator_set.addresses.clone(),
            },
            Coins::new(),
        )
        .should_succeed();

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
        sender: MOCK_ROUTE.address,
        destination_domain: MOCK_LOCAL_DOMAIN, // this should be our local domain
        recipient: contracts.hyperlane.warp.into(),
        body: TokenMessage {
            recipient: accounts.user1.address().into(),
            amount: Uint128::new(88),
            metadata: HexBinary::default(),
        }
        .encode(),
    }
    .encode();

    let message_id = raw_message.keccak256();
    let raw_metadata = mock_validator_set.sign(message_id).encode();

    suite
        .execute(
            &mut accounts.owner,
            contracts.hyperlane.mailbox,
            &mailbox::ExecuteMsg::Process {
                raw_message: raw_message.clone(),
                raw_metadata,
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
        .query_balance(&accounts.user1, denom.clone())
        .should_succeed_and_equal(Uint128::new(88));
}
