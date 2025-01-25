use {
    dango_testing::{create_recoverable_signature, setup_test, TestAccount},
    grug::{Addr, Coins, HashExt, HexByteArray, QuerierExt, ResultExt},
    hyperlane_types::{
        announcement_hash, domain_hash, eip191_hash,
        mailbox::{self, Domain},
        va::{self, VA_DOMAIN_KEY},
    },
    std::collections::{BTreeMap, BTreeSet},
};

struct MockAnnouncement {
    validator: HexByteArray<20>,
    signature: HexByteArray<65>,
    storage_location: String,
}

impl MockAnnouncement {
    fn new(
        account: &TestAccount,
        mailbox: Addr,
        local_domain: Domain,
        storage_location: &str,
    ) -> Self {
        // Retrieve validator address.
        let singing_key = account.first_signing_key();
        let public_key = singing_key.verifying_key().to_encoded_point(false);
        let pk_hash = (&public_key.as_bytes()[1..]).keccak256();
        let validator = &pk_hash[12..];

        // Create msg to sign.
        let message_hash = eip191_hash(announcement_hash(
            domain_hash(local_domain, mailbox.into(), VA_DOMAIN_KEY),
            storage_location,
        ));

        // Create signature.
        let (signature, recover_id) = create_recoverable_signature(singing_key, message_hash);
        let mut recoverable_signature = [0u8; 65];
        recoverable_signature[..64].copy_from_slice(&signature);
        recoverable_signature[64] = recover_id + 27; // Add 27 to recover_id.

        Self {
            validator: validator.try_into().unwrap(),
            signature: HexByteArray::from_inner(recoverable_signature),
            storage_location: storage_location.to_string(),
        }
    }
}

#[test]
fn test_announce() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let mailbox = contracts.hyperlane.mailbox;
    let local_domain = suite
        .query_wasm_smart(mailbox, mailbox::QueryConfigRequest {})
        .should_succeed()
        .local_domain;

    let va = contracts.hyperlane.va;

    // Create a working announcement.
    let announcement = MockAnnouncement::new(
        &accounts.user1,
        mailbox,
        local_domain,
        "Test/Storage/Location",
    );

    suite
        .execute(
            &mut accounts.user1,
            va,
            &va::ExecuteMsg::Announce {
                storage_location: announcement.storage_location.clone(),
                validator: announcement.validator,
                signature: announcement.signature,
            },
            Coins::new(),
        )
        .should_succeed();

    // Check that the validator was added to the validators.
    let mut validators_expected = BTreeSet::from([announcement.validator]);

    let validators_query = suite
        .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {})
        .should_succeed();
    assert_eq!(validators_query, validators_expected);

    // Check that the validator was added to the storage locations.
    let mut storage_locations_expected = BTreeMap::from([(
        announcement.validator,
        BTreeSet::from(["Test/Storage/Location".to_string()]),
    )]);

    let storage_locations_query = suite
        .query_wasm_smart(va, va::QueryAnnounceStorageLocationsRequest {
            validators: BTreeSet::from_iter([announcement.validator]),
        })
        .should_succeed();
    assert_eq!(storage_locations_query, storage_locations_expected);

    // Adding for second time same validator and same storage location;
    // there should be no changes.
    suite
        .execute(
            &mut accounts.user1,
            va,
            &va::ExecuteMsg::Announce {
                storage_location: announcement.storage_location,
                validator: announcement.validator,
                signature: announcement.signature,
            },
            Coins::new(),
        )
        .should_succeed();

    // Check that there are no changes.
    let validators_query = suite
        .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {})
        .should_succeed();
    assert_eq!(validators_query, validators_expected);

    let storage_locations_query = suite
        .query_wasm_smart(va, va::QueryAnnounceStorageLocationsRequest {
            validators: BTreeSet::from_iter([announcement.validator]),
        })
        .should_succeed();
    assert_eq!(storage_locations_query, storage_locations_expected);

    // Adding same validator with different storage location.
    let announcement_2 = MockAnnouncement::new(
        &accounts.user1,
        mailbox,
        local_domain,
        "Test/Storage/Location/2",
    );

    suite
        .execute(
            &mut accounts.user1,
            va,
            &va::ExecuteMsg::Announce {
                storage_location: announcement_2.storage_location.clone(),
                validator: announcement_2.validator,
                signature: announcement_2.signature,
            },
            Coins::new(),
        )
        .should_succeed();

    // Check that the validator was added to the validators.
    validators_expected.insert(announcement_2.validator);

    let validators_query = suite
        .query_wasm_smart(va, va::QueryAnnouncedValidatorsRequest {})
        .should_succeed();
    assert_eq!(validators_query, validators_expected);

    // Check that the storage location was added.
    let storage_locations_query = suite
        .query_wasm_smart(va, va::QueryAnnounceStorageLocationsRequest {
            validators: BTreeSet::from_iter([announcement.validator]),
        })
        .should_succeed();

    storage_locations_expected
        .get_mut(&announcement.validator)
        .unwrap()
        .insert(announcement_2.storage_location);

    assert_eq!(storage_locations_query, storage_locations_expected);

    // Adding a different validator.
    let announcement_3 = MockAnnouncement::new(
        &accounts.user2,
        mailbox,
        local_domain,
        "Test/Storage/Location/3",
    );

    suite
        .execute(
            &mut accounts.user2,
            va,
            &va::ExecuteMsg::Announce {
                storage_location: announcement_3.storage_location.clone(),
                validator: announcement_3.validator,
                signature: announcement_3.signature,
            },
            Coins::new(),
        )
        .should_succeed();

    // Check that the storage location was added.
    let storage_locations_query = suite
        .query_wasm_smart(va, va::QueryAnnounceStorageLocationsRequest {
            validators: BTreeSet::from_iter([announcement.validator, announcement_3.validator]),
        })
        .should_succeed();

    storage_locations_expected.insert(
        announcement_3.validator,
        BTreeSet::from([announcement_3.storage_location]),
    );

    assert_eq!(storage_locations_query, storage_locations_expected);

    // Creating a failing announcement for different local denom.
    let announcement = MockAnnouncement::new(
        &accounts.user1,
        mailbox,
        local_domain + 1,
        "Test/Storage/Location",
    );

    suite
        .execute(
            &mut accounts.user1,
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

#[test]
fn test_query() {
    let (suite, _, _, contracts) = setup_test();

    let va = contracts.hyperlane.va;
    let mailbox = contracts.hyperlane.mailbox;
    let local_domain = suite
        .query_wasm_smart(mailbox, mailbox::QueryConfigRequest {})
        .should_succeed()
        .local_domain;

    // Assert that the local domain is correct.
    let local_domain_query = suite
        .query_wasm_smart(va, va::QueryLocalDomainRequest {})
        .should_succeed();

    assert_eq!(local_domain_query, local_domain);

    // Assert mailbox is correct.
    let mailbox_query = suite
        .query_wasm_smart(va, va::QueryMailboxRequest {})
        .should_succeed();

    assert_eq!(mailbox_query, mailbox);
}
