use {
    core::time,
    dango_app::PythProposalPreparer,
    dango_testing::{setup_test, TestAccount},
    dango_types::{
        account_factory::Username,
        oracle::{ExecuteMsg, PriceSource, PythId, QueryPriceSourcesRequest, QueryPricesRequest},
    },
    grug::{btree_map, Addr, Addressable, Coins, Denom, ResultExt},
    std::{str::FromStr, thread::sleep},
};

#[test]
fn proposal_pyth() {
    let (mut suite, mut account, _, contracts) = setup_test();

    let price_ids = btree_map!(
        Denom::from_str("usdc").unwrap() => PriceSource::Pyth { id:  PythId::from_str("0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a").unwrap(), precision: 6 },
        Denom::from_str("btc").unwrap() => PriceSource::Pyth { id: PythId::from_str("0xc9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33").unwrap(), precision: 8 },
        Denom::from_str("eth").unwrap() => PriceSource::Pyth { id: PythId::from_str("0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace").unwrap(), precision: 18 },
    );

    // register the price sources
    suite
        .execute(
            &mut account.owner,
            contracts.oracle,
            &ExecuteMsg::RegisterPriceSources(price_ids.clone()),
            Coins::default(),
        )
        .should_succeed();

    // check if they are registered
    let res = suite
        .query_wasm_smart(contracts.oracle, QueryPriceSourcesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    assert_eq!(res, price_ids);

    // trigger the prepare proposal to update the prices
    suite.make_empty_block();

    // retrive the prices
    let prices1 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // await some time and assert that the timestamp are updated
    sleep(time::Duration::from_secs(2));

    suite.make_empty_block();

    let prices2 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // assert that the timestamp are updated
    for (denom, price) in prices1 {
        assert_ne!(price.timestamp, prices2.get(&denom).unwrap().timestamp);
    }
}

#[test]
fn proposal_pyth_create() {
    let feeder = TestAccount::new_random("feeder")
        .set_address(&btree_map!(Username::from_str("feeder").unwrap() => Addr::mock(1)));

    PythProposalPreparer::new(
        "dev-1".to_string(),
        feeder.address(),
        &feeder.sk.to_bytes(),
        feeder.username.to_string(),
    )
    .unwrap();
}
