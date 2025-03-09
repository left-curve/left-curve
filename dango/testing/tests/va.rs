use {
    dango_testing::{generate_random_key, setup_test},
    grug::{
        Addr, Addressable, CheckedContractEvent, Coins, HashExt, HexByteArray, Inner, JsonDeExt,
        QuerierExt, ResultExt, SearchEvent, UniqueVec, btree_set,
    },
    grug_crypto::Identity256,
    hyperlane_types::{
        announcement_hash, domain_hash, eip191_hash,
        mailbox::{self, Domain},
        va::{self, Announce, VA_DOMAIN_KEY},
    },
    k256::ecdsa::SigningKey,
    std::collections::{BTreeMap, BTreeSet},
};

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
        // We need the _uncompressed_ pubkey for deriving Ethereum address.
        let pk = sk.verifying_key().to_encoded_point(false).to_bytes();
        let pk_hash = (&pk[1..]).keccak256();
        let validator_address = &pk_hash[12..];

        // Create msg to sign.
        let message_hash = eip191_hash(announcement_hash(
            domain_hash(local_domain, mailbox.into(), VA_DOMAIN_KEY),
            storage_location,
        ));

        let (signature, recovery_id) = sk
            .sign_digest_recoverable(Identity256::from(message_hash.into_inner()))
            .unwrap();

        let mut packed = [0u8; 65];
        packed[..64].copy_from_slice(&signature.to_bytes());
        packed[64] = recovery_id.to_byte() + 27;

        Self {
            sk,
            validator: validator_address.try_into().unwrap(),
            signature: packed.into(),
            storage_location: storage_location.to_string(),
        }
    }
}

#[test]
fn test_announce() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let mut signer = accounts.user1;

    let mailbox = contracts.hyperlane.mailbox;
    let local_domain = suite
        .query_wasm_smart(mailbox, mailbox::QueryConfigRequest {})
        .should_succeed()
        .local_domain;

    let va = contracts.hyperlane.va;

    let mut validators_expected = BTreeSet::new();
    let mut storage_locations_expected = BTreeMap::new();

    // Create a working announcement.
    let announcement = MockAnnouncement::new(mailbox, local_domain, "Test/Storage/Location");

    // Adding a valid announcement.
    {
        suite
            .execute(
                &mut signer,
                va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location.clone(),
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                Coins::new(),
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
                sender: signer.address(),
                validator: announcement.validator,
                storage_location: announcement.storage_location.clone(),
            });

        // Check that the validator was added to the validators.
        validators_expected.insert(announcement.validator);

        suite
            .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the validator was added to the storage locations.
        storage_locations_expected.insert(
            announcement.validator,
            UniqueVec::new_unchecked(vec!["Test/Storage/Location".to_string()]),
        );

        suite
            .query_wasm_smart(va, va::QueryAnnouncedStorageLocationsRequest {
                validators: btree_set![announcement.validator],
            })
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Adding the same announcement again.
    {
        suite
            .execute(
                &mut signer,
                va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location,
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                Coins::new(),
            )
            .should_fail_with_error("duplicate data found!");
    }

    // Adding same validator with different storage location.
    {
        let announcement2 = MockAnnouncement::new_with_singing_key(
            announcement.sk,
            mailbox,
            local_domain,
            "Test/Storage/Location/2",
        );

        suite
            .execute(
                &mut signer,
                va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement2.storage_location.clone(),
                    validator: announcement2.validator,
                    signature: announcement2.signature,
                },
                Coins::new(),
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
                sender: signer.address(),
                validator: announcement2.validator,
                storage_location: announcement2.storage_location.clone(),
            });

        // Check there are no change in validators.
        suite
            .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the storage location was added.
        storage_locations_expected
            .get_mut(&announcement.validator)
            .unwrap()
            .try_push(announcement2.storage_location)
            .unwrap();

        suite
            .query_wasm_smart(va, va::QueryAnnouncedStorageLocationsRequest {
                validators: btree_set![announcement.validator],
            })
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Adding a different validator.
    {
        let announcement3 = MockAnnouncement::new(mailbox, local_domain, "Test/Storage/Location/3");

        suite
            .execute(
                &mut accounts.user2,
                va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement3.storage_location.clone(),
                    validator: announcement3.validator,
                    signature: announcement3.signature,
                },
                Coins::new(),
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
            .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {
                start_after: None,
                limit: None,
            })
            .should_succeed_and_equal(validators_expected.clone());

        // Check that the storage location was added.
        storage_locations_expected.insert(
            announcement3.validator,
            UniqueVec::new_unchecked(vec![announcement3.storage_location]),
        );

        suite
            .query_wasm_smart(va, va::QueryAnnouncedStorageLocationsRequest {
                validators: btree_set![announcement.validator, announcement3.validator],
            })
            .should_succeed_and_equal(storage_locations_expected.clone());
    }

    // Try adding a invalid announcement with different local domain
    // (should fail for pubkey mismatch).
    {
        let announcement =
            MockAnnouncement::new(mailbox, local_domain + 1, "Test/Storage/Location");

        suite
            .execute(
                &mut signer,
                va,
                &va::ExecuteMsg::Announce {
                    storage_location: announcement.storage_location,
                    validator: announcement.validator,
                    signature: announcement.signature,
                },
                Coins::new(),
            )
            .should_fail_with_error("pubkey mismatch");
    }
}

#[test]
fn test_query() {
    let (suite, _, _, contracts) = setup_test();

    // Assert mailbox is correct.
    suite
        .query_wasm_smart(contracts.hyperlane.va, va::QueryMailboxRequest {})
        .should_succeed_and_equal(contracts.hyperlane.mailbox);
}
