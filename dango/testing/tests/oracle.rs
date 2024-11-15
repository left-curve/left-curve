use {
    dango_testing::{setup_test, Accounts, TestSuite},
    dango_types::oracle::{
        self, ExecuteMsg, GuardianSet, PrecisionlessPrice, PriceSource, PythId, PythVaa,
        QueryPriceRequest, ATOM_USD_ID, BNB_USD_ID, DOGE_USD_ID, ETH_USD_ID, GUARDIANS_ADDRESSES,
        GUARDIAN_SETS_INDEX, SHIBA_USD_ID, SOL_USD_ID, TON_USD_ID, USDC_USD_ID, WBTC_USD_ID,
        XRP_USD_ID,
    },
    grug::{
        btree_map, Addr, Binary, Coins, Denom, Hash160, Inner, MockApi, NonEmpty, ResultExt,
        Udec128,
    },
    pyth_sdk::PriceFeed,
    std::{collections::BTreeMap, str::FromStr, thread, time::Duration},
};

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **6864578657006**
/// - publish_time: **1730804420**
const VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQBnC+7yOL2qsxrpxHzhTnaruVWTSfjBRIF7sk1bJUZzj3s7wZyytPTHtoxXFQaFFSCgVpCXeLdeHuN3ZM2LOvQMAAMOXaxpZYUuwjEhbN8yP3wfgSDdaFgiS0Abr1Hyf29BX1sYEEH82xUVspIdEv7DBves+XjKJWWnZ51De4KMmDqgAQR9ExeR/D3QbvfFarB73jLQ+QKGS0tb50229RyjKCHv2VbRJL5go04kePmSqLjqjhBn/IBx2Rr1W16DF9fKV2h+AAaPsmegjpPIfPIDZwqMcgvNfXqG77+8RYSH95azsCTMEFOaQVtJGJbjQUWdSrlqXukLgxIxf6yKdzp7sOBNFFVdAAhXAe1EFhONyQgWDnViECw7DbvmwNtjJ2xM/DslvZ2RJVA46pZ5St6IKyK2Ucqq/0Hu2nC1CEB39Rtcvu0Sm6DCAQpyh2KzwK+i9CtzyZNYfRFn+esWmnSHpoZrBYLgxayqtRIiTPetE3hudyHUxm4xk7CfcBrRD8uThsny1YHeiQpiAQtcR4XqjxUWHNLXsMaqaF3B/pskIjxVjWEiDkJCIpqoJFn8tktkDh00XREbZ68SUhUQQ1/S6icJLUIQt2Rf4cy5AAwJVyMi0NmjVs0X5NYzwO1Uk6Yfx96HQtibi9gPiCR4gXTW0udFzqvQ2u2xiiXonGjmaRMW86hm/6kx08d341PTAQ2ypzyZJiPhPZAo4I2IJtdjkq72uyR4lL1kqaIGupLxtCq36i1tD61Yjt3HRruBuVvHqjC60xDvWIVQL6UAHAu9AQ7wH5SeZ1ra473yrfVGIEtuGSh0iITJ3Tnzh+4IJMdnvjFARCrxHLmne50gjYcG+CQYSHl/TJ+fElFtiDx43ouGABCT8qRJAJYpusR2A1mGXDX/oBSq0NoaKKr7u4c8zLDsLWUudBRRVkDS4281f+GuQupa5eRPKdDHXt40lFY5V+FWABEcD+ka2buu8h4ZAK9gWcOhe9Ms0COktqchnwS3oZV7lXXcZM1K+LKc+gKshOln7r3JC1UrkcjJa6gy9v5Ka9YTARIYLaAd0TkttKtK9hoALKRTkEqpqgtvBLqJA9qW1UDYoAZksJo2X0th7lFdIZJQsCIkDxqedbuS1H7EQ7Im6XUHAGcp+sQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFcADhAUFVV1YAAAAAAAp8yzcAACcQXNrQIBXy4Cs6ul3jv4wlMishtwkBAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAY+SMW67gAAAAI/1aJY////+AAAAABnKfrEAAAAAGcp+sQAAAY+IAqHoAAAAAIKJkDECnJa8p4N3HJckOG0/XBHQ4HSCFfFvzHVwHvYJ9V5NPKGlOHwUp0GbOXWbNIMhSmoX+hk8FUMlP6NlHHbf8S2YxVixm+nMOOrhtH9+3bMQQh26XE6/E5UIoNgScjtRRQ32qtHxrU1ezhAhHmTAAD07E8S/ACc8F8xjDAZgLgjSFLHptczUSe1wR5IrrbZQRQhERagNdCcBUp8S5wl7VAQBPqprw5ZZ6dvI0y/P8UaldqRoa8eN47BbGvH/12oNzfcUiLjHCFciAwc";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **7131950295749**
/// - publish_time: **1730209108**
const VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQBLJRnF435tmWmnpCautCMOcWFhH0neObVk2iw/qtQ/jX44qUBV+Du+woo5lWLrE1ttnAPfwv9aftKy/r0pz0OdAQP25Bjy5Hx3MaOEF49sx+OrA6fxSNtBIxEkZ/wqznQAvlNE86loIz2osKoAWYeCg9FjU/8A2CmZZhcyXb4Cf+beAQSN829+7wKOw6tdMnKwtiYKdXL1yo1uP10iZ3EhU2M4cxrD0xYKA0pkb9hmhRo+zHrOY9pyTGXAsz7FjlI+gvgCAQa5MiGBgMRLFGW0fTd+bqc+isCQDbhgm/99yNkVaDt40ASST8CfH5zp4Xim5l5Yhs+/HMpeFSuTNULeDXsTO2FaAAjaPzeC8Bie6n154BaKA+45xn0lDa0epmVZs16zVCkKczSUNVG5e5VZe6N8edT+dVicoZYT9tgHJn2WDIjcpRv7AAsc0fdXE42zolp1Dhg1XVL5oe6NeTZi2Beu2ecv5FkvtCwm9dytTv6C359wJqUZLbZVaqOU9CEVbBvTzbKAm/tQAAx12qSCdkLtlJZAmhhrCvW56375q1Dy74L417r+GhDgYRqPCNWyaY7azRFfOwahxc9ECZgHj1aJg0bk395+JhTnAQ2K/IC6aRcSpPd+SfbWnfPtdJTdJFw5QCS50FbBfxxmqBTcG8E8fyYyCz5SGC8rtXgrBi+cQZe8FgW4CoLXXxC+AQ7TotPy0p9aHpwlIrXvu9B2nThByrwd4icwnOfQsUDHcG65PXWvu9nc1o5EK6SImnv+AmIu+RID2MnyTavsGEMpAA/XdQHG8mkgdWlZ1w7fg2MBs3fa0VxIlKc1DuaBdZVZEjrnB4gE15oqMZ21Bt8ji6r6J+ar/9K46EUeYC2t6CuBARDpRTI9ZZlh0MvxIbxRkuAgtRTv8oNrSz4sQJMNbhWdswTmqQQMZjtdJwGWepaAGhnEiuF/JgIr20AnDxCWbolgABGwVILVFDCHnLV54/bIdXUEiigPZvsKcDxLpOoJ722xZT1cXwXoBmwQ2lXQxGOjyj8VvgAt2kZJNbGc77+pmsqdABIFwK9Dc5BLxz+dXztA5bPMcEKkfZ18t7HPZ9BVQN7f1Cw4XcBZDSRR0MM6tqeBYvLJZhDMbt2Ax0m0+RlzQTZyAWcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo1AUFVV1YAAAAAAApl86sAACcQTdtYrFsURmdX9JeZM/nLGOdGy18BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZ8iV0qxQAAAAIvYnVX////+AAAAABnIOVUAAAAAGcg5VQAAAZ3rChYAAAAAAIykC3MCknCJZOvI3H3Ijt5NftDL77S253kTxg9ywpWvf3kzbZeQqXixw7K/fcAEWCww773jqhfS4CdRyUc38SMv+DhHywJbnUSyzFEWOTBVmVuvEtt6xWOTDMifAi8cAX0cBtZOyeIeLytWSqkMVYhtbm0gKCLnjtBEKLg/zEHSL48Ndm9VTihIpe8REto4Pf2MjlxRY6Smgw2TMZCJTCEj2869KzQsQhVSH4VmOJNJpevlYaqeFmJ7WDOC1tFWrVulGSZ/nIt63NKB+JP";

pub const PYTH_URL: &str = "https://hermes.pyth.network";

fn setup_oracle_test(denoms: BTreeMap<Denom, PriceSource>) -> (TestSuite, Accounts, Addr) {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    // init a new oracle
    let code_hash = suite.query_contract(&contracts.oracle).unwrap().code_hash;

    let oracle = suite
        .instantiate(
            &mut accounts.owner,
            code_hash,
            &oracle::InstantiateMsg {
                guardian_sets: btree_map! {
                    GUARDIAN_SETS_INDEX => GuardianSet {
                        addresses: GUARDIANS_ADDRESSES
                            .into_iter()
                            .map(|addr| {
                                let bytes = Binary::from_str(addr)
                                    .unwrap()
                                    .into_inner()
                                    .try_into()
                                    .unwrap();
                                Hash160::from_inner(bytes)
                            })
                            .collect(),
                        expiration_time: None,
                    },
                },
            },
            "salt",
            None,
            None,
            Coins::default(),
        )
        .should_succeed()
        .address;

    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::RegisterPriceSources(denoms),
            Coins::default(),
        )
        .should_succeed();

    (suite, accounts, oracle)
}

#[test]
fn oracle() {
    let precision = 8;
    let btc_denom = Denom::from_str("bridge/btc").unwrap();

    let (mut suite, mut accounts, oracle) = setup_oracle_test(btree_map! {
        btc_denom.clone() => PriceSource::Pyth { id: WBTC_USD_ID, precision }
    });

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_1).unwrap()
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest {
                denom: btc_denom.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("68645.78657006").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("68638.95300000").unwrap()
        );

        assert_eq!(current_price.precision(), precision);

        assert_eq!(current_price.timestamp, 1730804420);
    }

    // Push an updated_price
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_2).unwrap()
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest {
                denom: btc_denom.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("68690.52261900").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("68639.54100000").unwrap()
        );

        assert_eq!(current_price.timestamp, 1730804507);
    }

    // Push an outdated price. it should not be updated
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_1).unwrap()
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest { denom: btc_denom })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("68690.52261900").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("68639.54100000").unwrap()
        );

        assert_eq!(current_price.timestamp, 1730804507);
    }
}

#[test]
fn double_vaas() {
    let mut last_btc_vaa: Option<PriceFeed> = None;
    let mut last_eth_vaa: Option<PriceFeed> = None;

    let btc_denom = Denom::from_str("bridge/btc").unwrap();
    let eth_denom = Denom::from_str("bridge/eth").unwrap();

    let (mut suite, mut accounts, oracle) = setup_oracle_test(btree_map! {
        btc_denom.clone() => PriceSource::Pyth { id: WBTC_USD_ID, precision: 8 },
        eth_denom.clone() => PriceSource::Pyth { id: ETH_USD_ID, precision: 8 },
    });

    for _ in 0..5 {
        // get 2 separate vaa
        let btc_vaas_raw = get_latest_vaas([WBTC_USD_ID]).unwrap();
        let eth_vaas_raw = get_latest_vaas([ETH_USD_ID]).unwrap();

        let btc_vaa = PythVaa::new(&MockApi, btc_vaas_raw[0].clone().into_inner())
            .unwrap()
            .unverified()[0];
        let eth_vaa = PythVaa::new(&MockApi, eth_vaas_raw[0].clone().into_inner())
            .unwrap()
            .unverified()[0];

        // update last btc vaa
        {
            if let Some(last_btc_vaa) = &mut last_btc_vaa {
                if btc_vaa.get_price_unchecked().publish_time
                    > last_btc_vaa.get_price_unchecked().publish_time
                {
                    last_btc_vaa.clone_from(&btc_vaa);
                }
            } else {
                last_btc_vaa = Some(btc_vaa);
            }
        }

        // update last eth vaa
        {
            if let Some(last_eth_vaa) = &mut last_eth_vaa {
                if eth_vaa.get_price_unchecked().publish_time
                    > last_eth_vaa.get_price_unchecked().publish_time
                {
                    last_eth_vaa.clone_from(&eth_vaa);
                }
            } else {
                last_eth_vaa = Some(eth_vaa);
            }
        }

        // update price feeds
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(
                    [btc_vaas_raw, eth_vaas_raw].concat(),
                )),
                Coins::default(),
            )
            .should_succeed();

        // check btc price
        {
            let current_price = suite
                .query_wasm_smart(oracle, QueryPriceRequest {
                    denom: btc_denom.clone(),
                })
                .unwrap();

            assert_eq!(
                current_price.timestamp,
                last_btc_vaa
                    .unwrap()
                    .get_price_unchecked()
                    .publish_time
                    .unsigned_abs()
            );
            assert_eq!(
                current_price.humanized_price,
                PrecisionlessPrice::try_from(last_btc_vaa.unwrap())
                    .unwrap()
                    .humanized_price
            );

            assert_eq!(
                current_price.humanized_ema,
                PrecisionlessPrice::try_from(last_btc_vaa.unwrap())
                    .unwrap()
                    .humanized_ema
            );
        }

        // check eth price
        {
            let current_price = suite
                .query_wasm_smart(oracle, QueryPriceRequest {
                    denom: eth_denom.clone(),
                })
                .unwrap();

            assert_eq!(
                current_price.timestamp,
                last_eth_vaa
                    .unwrap()
                    .get_price_unchecked()
                    .publish_time
                    .unsigned_abs()
            );
            assert_eq!(
                current_price.humanized_price,
                PrecisionlessPrice::try_from(last_eth_vaa.unwrap())
                    .unwrap()
                    .humanized_price
            );

            assert_eq!(
                current_price.humanized_ema,
                PrecisionlessPrice::try_from(last_eth_vaa.unwrap())
                    .unwrap()
                    .humanized_ema
            );
        }

        // sleep for 1 second
        thread::sleep(Duration::from_secs(1));
    }
}

#[test]
fn multiple_vaas() {
    let id_denoms = btree_map! {
        WBTC_USD_ID  => Denom::from_str("bridge/btc").unwrap() ,
        ETH_USD_ID   => Denom::from_str("bridge/eth").unwrap() ,
        USDC_USD_ID  => Denom::from_str("bridge/usdc").unwrap() ,
        SOL_USD_ID   => Denom::from_str("bridge/sol").unwrap() ,
        ATOM_USD_ID  => Denom::from_str("bridge/atom").unwrap() ,
        BNB_USD_ID   => Denom::from_str("bridge/bnb").unwrap() ,
        DOGE_USD_ID  => Denom::from_str("bridge/doge").unwrap() ,
        XRP_USD_ID   => Denom::from_str("bridge/xrp").unwrap() ,
        TON_USD_ID   => Denom::from_str("bridge/ton").unwrap() ,
        SHIBA_USD_ID => Denom::from_str("bridge/shiba").unwrap(),
    };

    let denom_price_sources = id_denoms
        .iter()
        .map(|(id, denom)| {
            (denom.clone(), PriceSource::Pyth {
                id: *id,
                precision: 8,
            })
        })
        .collect();

    let (mut suite, mut accounts, oracle) = setup_oracle_test(denom_price_sources);

    let mut last_price_feeds = id_denoms
        .keys()
        .map(|id| (*id, None))
        .collect::<BTreeMap<_, Option<PriceFeed>>>();

    for _ in 0..5 {
        let vaas_raw = get_latest_vaas(id_denoms.keys()).unwrap();

        let vaas = vaas_raw
            .iter()
            .map(|vaa_raw| PythVaa::new(&MockApi, vaa_raw.clone().into_inner()).unwrap())
            .collect::<Vec<_>>();

        // Update last price feeds
        for vaa in vaas {
            for price_feed in vaa.unverified() {
                let last_price_feed = last_price_feeds
                    .get_mut(&PythId::from_str(&price_feed.id.to_string()).unwrap())
                    .unwrap();

                if let Some(last_price_feed) = last_price_feed {
                    if price_feed.get_price_unchecked().publish_time
                        > last_price_feed.get_price_unchecked().publish_time
                    {
                        last_price_feed.clone_from(&price_feed);
                    }
                } else {
                    *last_price_feed = Some(price_feed);
                }
            }
        }

        // Check if all prices has been fetched
        for v in last_price_feeds.values() {
            assert!(v.is_some());
        }

        // Push all prices
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vaas_raw)),
                Coins::default(),
            )
            .should_succeed();

        // Check all prices
        for (denom, last_price_feed) in &last_price_feeds {
            let denom = id_denoms.get(denom).unwrap();

            let current_price = suite
                .query_wasm_smart(oracle, QueryPriceRequest {
                    denom: denom.clone(),
                })
                .unwrap();

            assert_eq!(
                current_price.timestamp,
                last_price_feed
                    .unwrap()
                    .get_price_unchecked()
                    .publish_time
                    .unsigned_abs()
            );
            assert_eq!(
                current_price.humanized_price,
                PrecisionlessPrice::try_from(last_price_feed.unwrap())
                    .unwrap()
                    .humanized_price
            );

            assert_eq!(
                current_price.humanized_ema,
                PrecisionlessPrice::try_from(last_price_feed.unwrap())
                    .unwrap()
                    .humanized_ema
            );
        }

        // sleep for 1 second
        thread::sleep(Duration::from_secs(1));
    }
}

/// Return JSON string of the latest VAA from Pyth network.
fn get_latest_vaas<I>(ids: I) -> reqwest::Result<Vec<Binary>>
where
    I: IntoIterator,
    I::Item: ToString,
{
    let ids = ids
        .into_iter()
        .map(|id| ("ids[]", id.to_string()))
        .collect::<Vec<_>>();

    reqwest::blocking::Client::new()
        .get(format!("{PYTH_URL}/api/latest_vaas"))
        .query(&ids)
        .send()?
        .json()
}
