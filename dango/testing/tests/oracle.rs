use {
    dango_testing::{TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        constants::{btc, eth},
        oracle::{ExecuteMsg, PriceSource, QueryPriceRequest, QueryTrustedSignersRequest},
    },
    grug::{
        Addr, Binary, ByteArray, Coins, NonEmpty, QuerierExt, ResultExt, Timestamp, Udec128,
        btree_map,
    },
    grug_app::NaiveProposalPreparer,
    pyth_types::{Channel, LeEcdsaMessage, constants::LAZER_TRUSTED_SIGNER},
    std::str::FromStr,
};

fn setup_oracle_test() -> (TestSuite<NaiveProposalPreparer>, TestAccounts, Addr) {
    let (suite, accounts, _, contracts, _) = setup_test_naive(Default::default());
    (suite, accounts, contracts.oracle)
}

#[test]
fn pyth_lazer() {
    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    let message = LeEcdsaMessage {
        payload: Binary::from_inner(vec![
            117, 211, 199, 147, 192, 211, 105, 236, 97, 63, 6, 0, 1, 2, 1, 0, 0, 0, 2, 0, 62, 69,
            110, 163, 70, 10, 0, 0, 4, 248, 255, 2, 0, 0, 0, 2, 0, 142, 173, 202, 117, 97, 0, 0, 0,
            4, 248, 255,
        ]),
        signature: ByteArray::from_inner([
            186, 96, 166, 26, 76, 188, 9, 187, 138, 228, 131, 44, 114, 155, 181, 87, 138, 140, 135,
            77, 124, 146, 141, 138, 208, 195, 55, 185, 146, 227, 205, 115, 68, 98, 149, 231, 81,
            228, 139, 163, 157, 174, 155, 188, 121, 0, 136, 119, 109, 122, 177, 102, 73, 190, 130,
            37, 171, 253, 166, 18, 185, 152, 53, 136,
        ]),
        recovery_id: 0,
    };

    let trusted_signer = Binary::from_str(LAZER_TRUSTED_SIGNER).unwrap();

    // Set price source in oracle
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! {
                btc::DENOM.clone() => PriceSource::Pyth { id: 1, precision: 8, channel:Channel::RealTime },
                eth::DENOM.clone() => PriceSource::Pyth { id: 2, precision: 18 , channel:Channel::RealTime },
            }),
            Coins::default(),
        )
        .should_succeed();

    // The trusted signer was set in genesis. For the purpose of this test,
    // remove it for now, to test what happens if we submit price data from an
    // untrusted signer.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RemoveTrustedSigner {
                public_key: trusted_signer.clone(),
            },
            Coins::default(),
        )
        .should_succeed();

    // Try to feed price from Pyth Lazer. Should fail because the signer is not trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message.clone()])),
            Coins::default(),
        )
        .should_fail_with_error("signer is not trusted");

    // Get current time
    let current_time = suite.block.timestamp;

    // Set the signer as trusted but with a timestamp in the past.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterTrustedSigner {
                public_key: trusted_signer.clone(),
                expires_at: current_time - grug::Duration::from_seconds(60), // 1 minute ago
            },
            Coins::default(),
        )
        .should_succeed();

    // Query the trusted signers
    let trusted_signers = suite
        .query_wasm_smart(oracle, QueryTrustedSignersRequest {
            limit: None,
            start_after: None,
        })
        .unwrap();
    assert_eq!(trusted_signers.len(), 1);

    let (signer, timestamp) = trusted_signers.iter().next().unwrap();
    assert_eq!(signer, &trusted_signer);
    assert_eq!(
        timestamp,
        &(current_time - grug::Duration::from_seconds(60))
    );

    // Try to feed price from Pyth Lazer. Should fail because the signer is no longer trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message.clone()])),
            Coins::default(),
        )
        .should_fail_with_error("signer is no longer trusted");

    // Set the signer as trusted but with a timestamp in the future.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterTrustedSigner {
                public_key: trusted_signer.clone(),
                expires_at: current_time + grug::Duration::from_seconds(60), // 1 minute from now
            },
            Coins::default(),
        )
        .should_succeed();

    // Try to feed price from Pyth Lazer. Should succeed because the signer is trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message])),
            Coins::default(),
        )
        .should_succeed();

    // Query the BTC price
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: btc::DENOM.clone(),
        })
        .unwrap();

    assert_eq!(
        price.humanized_price,
        Udec128::from_str("112985.05901374").unwrap()
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1758539671000000));

    // Query the ETH price
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: eth::DENOM.clone(),
        })
        .unwrap();

    assert_eq!(
        price.humanized_price,
        Udec128::from_str("4185.88044686").unwrap()
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1758539671000000));
}
