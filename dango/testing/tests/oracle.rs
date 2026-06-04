use {
    dango_order_book::UsdPrice,
    dango_testing::{TestAccounts, TestSuiteNaive, setup_test_naive},
    dango_types::{
        constants::{eth, perp_btc},
        oracle::{
            ExecuteMsg, PriceConfig, PriceSource, QueryPriceRequest, QueryTrustedSignersRequest,
        },
    },
    grug_math::Dec128_6,
    grug_types::{
        Addr, Binary, ByteArray, Coins, Duration, NonEmpty, QuerierExt, ResultExt, Timestamp,
        btree_map,
    },
    pyth_types::{Channel, LeEcdsaMessage, MarketSession, constants::LAZER_TRUSTED_SIGNER},
    std::str::FromStr,
};

fn setup_oracle_test() -> (TestSuiteNaive, TestAccounts, Addr) {
    let (suite, accounts, _, contracts, _) = setup_test_naive(Default::default());
    (suite, accounts, contracts.oracle)
}

#[tokio::test]
async fn pyth_lazer() {
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
                perp_btc::DENOM.clone() => PriceConfig::Single(PriceSource { id: 1, channel: Channel::RealTime }),
                eth::DENOM.clone() => PriceConfig::Single(PriceSource { id: 2, channel: Channel::RealTime }),
            }),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Genesis registers the mock signer. Remove it so this test starts with
    // no trusted signers — we explicitly manage signer trust below.
    let mock_pubkey = dango_testing::mock_pyth_trusted_signer();

    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RemoveTrustedSigner {
                public_key: mock_pubkey,
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Try to feed price from Pyth Lazer. Should fail because the signer is not trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message.clone()])),
            Coins::default(),
        )
        .await
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
                expires_at: current_time - Duration::from_seconds(60), // 1 minute ago
            },
            Coins::default(),
        )
        .await
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
    assert_eq!(timestamp, &(current_time - Duration::from_seconds(60)));

    // Try to feed price from Pyth Lazer. Should fail because the signer is no longer trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message.clone()])),
            Coins::default(),
        )
        .await
        .should_fail_with_error("signer is no longer trusted");

    // Set the signer as trusted but with a timestamp in the future.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterTrustedSigner {
                public_key: trusted_signer.clone(),
                expires_at: current_time + Duration::from_seconds(60), // 1 minute from now
            },
            Coins::default(),
        )
        .await
        .should_succeed();

    // Try to feed price from Pyth Lazer. Should succeed because the signer is trusted.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message])),
            Coins::default(),
        )
        .await
        .should_succeed();

    // Query the BTC price
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: perp_btc::DENOM.clone(),
        })
        .unwrap();

    assert_eq!(
        price.humanized_price,
        UsdPrice::new(Dec128_6::from_str("112985.059013").unwrap())
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1758539671000000));
    // The captured payload predates our subscription to `MarketSession`,
    // so the property is absent and the parser falls back to `Other`.
    assert_eq!(price.market_session, MarketSession::Other);

    // Query the ETH price
    let price = suite
        .query_wasm_smart(oracle, QueryPriceRequest {
            denom: eth::DENOM.clone(),
        })
        .unwrap();

    assert_eq!(
        price.humanized_price,
        UsdPrice::new(Dec128_6::from_str("4185.880446").unwrap())
    );
    assert_eq!(price.timestamp, Timestamp::from_micros(1758539671000000));
    assert_eq!(price.market_session, MarketSession::Other);
}
