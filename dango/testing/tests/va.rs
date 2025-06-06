use {
    dango_testing::{generate_random_key, setup_test},
    dango_types::constants::{dango, usdc},
    grug::{
        Addr, Addressable, CheckedContractEvent, Coins, HexByteArray, JsonDeExt, QuerierExt,
        ResultExt, SearchEvent, UniqueVec, btree_set, coins,
    },
    hyperlane_testing::{constants::MOCK_HYPERLANE_LOCAL_DOMAIN, eth_utils},
    hyperlane_types::{
        announcement_hash, domain_hash, eip191_hash,
        mailbox::Domain,
        va::{self, Announce, VA_DOMAIN_KEY},
    },
    k256::ecdsa::SigningKey,
    std::collections::{BTreeMap, BTreeSet},
};

const ANNOUNCE_FEE_PER_BYTE: u128 = 100;

struct MockAnnouncement {
    sk: SigningKey,
    validator: HexByteArray<20>,
    signature: HexByteArray<65>,
    storage_location: String,
}

impl MockAnnouncement {
    fn new(mailbox: Addr, local_domain: Domain, storage_location: &str) -> Self {
        let (sk, _) = generate_random_key();

        Self::new_with_singing_key(sk, mailbox, local_domain, storage_location)
    }

    fn new_with_singing_key(
        sk: SigningKey,
        mailbox: Addr,
        local_domain: Domain,
        storage_location: &str,
    ) -> Self {
        // Derive the validator's Ethereum address.
        let validator_address = eth_utils::derive_address(sk.verifying_key());

        // Create message to sign.
        let message_hash = eip191_hash(announcement_hash(
            domain_hash(local_domain, mailbox.into(), VA_DOMAIN_KEY),
            storage_location,
        ));

        // Sign the message.
        let signature = eth_utils::sign(message_hash, &sk);

        Self {
            sk,
            validator: validator_address.into(),
            signature: signature.into(),
            storage_location: storage_location.to_string(),
        }
    }
}

#[test]
fn test_announce() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test(Default::default());

    let mut validators_expected = BTreeSet::new();
    let mut storage_locations_expected = BTreeMap::new();

    // Create a working announcement.
    let announcement = MockAnnouncement::new(
        contracts.hyperlane.mailbox,
        MOCK_HYPERLANE_LOCAL_DOMAIN,
        "Test/Storage/Location",
    );
    let announce_fee = ANNOUNCE_FEE_PER_BYTE * announcement.storage_location.len() as u128;

    // Announce without sending announce fee.
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                Coins::new(),
            )
            .should_fail_with_error("invalid payment");
    }

    // Announce with an incorrect payment (wrong denom)
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! { dango::DENOM.clone() => announce_fee },
            )
            .should_fail_with_error("invalid payment");
    }

    // Announce with an incorrect payment (multiple denoms)
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! {
                    dango::DENOM.clone() => announce_fee,
                    usdc::DENOM.clone()  => announce_fee,
                },
            )
            .should_fail_with_error("invalid payment");
    }

    // Announce with an insufficient payment.
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee - 1 },
            )
            .should_fail_with_error("insufficient validator announce fee");
    }

    // Adding a valid announcement.
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee },
            )
            .should_succeed()
            .events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|evt| evt.ty == "validator_announcement")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<Announce>()
            .should_succeed_and_equal(Announce {
                sender: accounts.owner.address(),
                validator: announcement.validator,
                storage_location: announcement.storage_location.clone(),
            });

        // Check that the validator was added to the validators.
        validators_expected.insert(announcement.validator);

        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedValidatorsRequest {
                    start_after: None,
                    limit: None,
                },
            )
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the validator was added to the storage locations.
        storage_locations_expected.insert(
            announcement.validator,
            UniqueVec::new_unchecked(vec!["Test/Storage/Location".to_string()]),
        );

        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedStorageLocationsRequest {
                    validators: btree_set![announcement.validator],
                },
            )
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Adding the same announcement again.
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location,
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee },
            )
            .should_fail_with_error("duplicate data found!");
    }

    // Adding same validator with different storage location.
    {
        let announcement2 = MockAnnouncement::new_with_singing_key(
            announcement.sk,
            contracts.hyperlane.mailbox,
            MOCK_HYPERLANE_LOCAL_DOMAIN,
            "Test/Storage/Location/2",
        );

        let announce_fee2 = ANNOUNCE_FEE_PER_BYTE * announcement2.storage_location.len() as u128;

        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement2.storage_location.clone(),
                    validator: announcement2.validator,
                    signature: announcement2.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee2 },
            )
            .should_succeed()
            .events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|evt| evt.ty == "validator_announcement")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<Announce>()
            .should_succeed_and_equal(Announce {
                sender: accounts.owner.address(),
                validator: announcement2.validator,
                storage_location: announcement2.storage_location.clone(),
            });

        // Check there are no change in validators.
        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedValidatorsRequest {
                    start_after: None,
                    limit: None,
                },
            )
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the storage location was added.
        storage_locations_expected
            .get_mut(&announcement.validator)
            .unwrap()
            .try_push(announcement2.storage_location)
            .unwrap();

        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedStorageLocationsRequest {
                    validators: btree_set![announcement.validator],
                },
            )
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Adding a different validator.
    {
        let announcement3 = MockAnnouncement::new(
            contracts.hyperlane.mailbox,
            MOCK_HYPERLANE_LOCAL_DOMAIN,
            "Test/Storage/Location/3",
        );

        let announce_fee3 = ANNOUNCE_FEE_PER_BYTE * announcement3.storage_location.len() as u128;

        suite
            .execute(
                &mut accounts.user2,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement3.storage_location.clone(),
                    validator: announcement3.validator,
                    signature: announcement3.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee3 },
            )
            .should_succeed()
            .events
            .search_event::<CheckedContractEvent>()
            .with_predicate(|evt| evt.ty == "validator_announcement")
            .take()
            .one()
            .event
            .data
            .deserialize_json::<Announce>()
            .should_succeed_and_equal(Announce {
                sender: accounts.user2.address(),
                validator: announcement3.validator,
                storage_location: announcement3.storage_location.clone(),
            });

        // Check that the validator was added to the validators.
        validators_expected.insert(announcement3.validator);

        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedValidatorsRequest {
                    start_after: None,
                    limit: None,
                },
            )
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the storage location was added.
        storage_locations_expected.insert(
            announcement3.validator,
            UniqueVec::new_unchecked(vec![announcement3.storage_location]),
        );

        suite
            .query_wasm_smart(
                contracts.hyperlane.va,
                va::QueryAnnouncedStorageLocationsRequest {
                    validators: btree_set![announcement.validator, announcement3.validator],
                },
            )
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Try adding a invalid announcement with different local domain
    // (should fail for pubkey mismatch).
    {
        let announcement = MockAnnouncement::new(
            contracts.hyperlane.mailbox,
            MOCK_HYPERLANE_LOCAL_DOMAIN + 1,
            "Test/Storage/Location",
        );

        let announce_fee = ANNOUNCE_FEE_PER_BYTE * announcement.storage_location.len() as u128;

        suite
            .execute(
                &mut accounts.owner,
                contracts.hyperlane.va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location,
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                coins! { usdc::DENOM.clone() => announce_fee },
            )
            .should_fail_with_error("pubkey mismatch");
    }
}

#[test]
fn test_query() {
    let (suite, _, _, contracts, _) = setup_test(Default::default());

    // Assert mailbox is correct.
    suite
        .query_wasm_smart(contracts.hyperlane.va, va::QueryMailboxRequest {})
        .should_succeed_and_equal(contracts.hyperlane.mailbox);
}
