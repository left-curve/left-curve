use {
    dango_testing::{TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        constants::{btc, eth},
        oracle::{ExecuteMsg, PriceSource, QueryPriceRequest, QueryTrustedSignersRequest},
    },
    grug::{
        Addr, Binary, Coins, EncodedBytes, NonEmpty, QuerierExt, ResultExt, Timestamp, Udec128,
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

// #[ignore = "work in progress"]
#[test]
fn pyth_lazer() {
    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    let message = LeEcdsaMessage {
        payload: vec![
            117, 211, 199, 147, 176, 69, 182, 116, 186, 60, 6, 0, 1, 2, 1, 0, 0, 0, 2, 0, 177, 106,
            175, 92, 86, 10, 0, 0, 4, 248, 255, 2, 0, 0, 0, 2, 0, 149, 185, 181, 48, 97, 0, 0, 0,
            4, 248, 255,
        ],
        signature: EncodedBytes::from_inner([
            130, 238, 159, 50, 90, 235, 146, 66, 22, 150, 217, 47, 21, 202, 76, 230, 207, 142, 241,
            194, 185, 61, 34, 194, 86, 164, 48, 50, 40, 197, 158, 129, 11, 220, 18, 70, 38, 166,
            191, 150, 182, 201, 45, 201, 18, 30, 187, 23, 31, 124, 182, 203, 141, 24, 28, 162, 91,
            199, 156, 252, 42, 49, 222, 140,
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
                btc::DENOM.clone() => PriceSource::PythLazer { id: 1, precision: 6, channel:Channel::RealTime },
                eth::DENOM.clone() => PriceSource::PythLazer { id: 2, precision: 18 , channel:Channel::RealTime },
            }),
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
            &ExecuteMsg::SetTrustedSigner {
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
            &ExecuteMsg::SetTrustedSigner {
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
        Udec128::from_str("113660.38465201").unwrap()
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1755621379950000));

    // Query the ETH price
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: eth::DENOM.clone(),
        })
        .unwrap();

    assert_eq!(
        price.humanized_price,
        Udec128::from_str("4174.29043605").unwrap()
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1755621379950000));
}
