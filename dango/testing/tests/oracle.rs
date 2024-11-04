use {
    dango_testing::setup_test,
    dango_types::oracle::{
        ExecuteMsg, PrecisionlessPrice, PriceSource, PythId, PythVaa, QueryPriceRequest,
    },
    grug::{btree_map, Binary, Coins, Denom, Inner, MockApi, ResultExt, Udec128},
    pyth_sdk::PriceFeed,
    std::{collections::BTreeMap, str::FromStr},
};

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **6984382159562**
/// - publish_time: **1730157441**
const VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQBkMyJzGWOwAlhd3NDvcYJvct5KACRi6oi9InIE/PYqXh1z92MOXFyFPGP5y9uOpubgMIvUh/pa5aXsM/z+aaCdAALKQlwSVB5YIQ/C0NuqXqam0fAAQYUJeBe+G7rjnv7UXhHRIqNiqCvTE1ygz3zUztg07pqoYahCI7SlqI23hHizAAPG7cQdoENAUMDgYC1znnRkG8NUDS/Yzlxb3Krl/fKDUjpgKM2ZEB5HD11bCTzIhPHTI8KQxIDbyKxF6o4cwf5QAAQxrIWXQX0Bx9/lDEDfFOOqRU6LwZhFMmiDwUedUxsIvR73V/yfZKNtObHA0O9McjdTo1JibRqnbNqw6H8hw4/JAAax4DOJ/M8yxbIk88rV0n8sttzelXPuMnnJCXV2CFpwlSqYu0cQ+gmWvfjK/zJSFKHhNF0N7wzOX9J/bghUeQ8nAQgJ7BPYtJo/qowTuQfDCa4ZHIhLjC9frRQh3/UWLrxosG5xWODfYWtpDLKwfmi2gjMV4PIMUdhwZLyMDfZIqR6MAQrB/IQ438iz+1cgU+i8ij7eB5+MeUxcV0ukQhJW/0nwVCm234OqZ+ES3fNPIpWHRo4nq5ZVCdX4ZE3MF+SjZIW2AAu4DFxPpw3tokuOP6z2jNk9AFzjC/WUqlZaIx+6Se5ZeGr4chhEh2IiwChhSUJnGsKtkXHSqTuLZpXf8QZ+ZiRFAAz9XiWxbiOvw6E4+I/0JRutYrALssiRNYBah4I1QzYSU1gIAeMEHz2jvMX9lGGZMfS/uJrv1VtW9UCJMxMCUqgOAA2Hkv95hjyj6toIigG6PyEpzzoJE3ZVqI92F2kWoGSE0l/7aV/sz6jhRl8udbq/Mqu+i9wpbUZqa/ZUCFFi0NLSAQ5s3Le7hPfK1QnMOU8eWkJqiy/XL+remqBwR92Omm8FFANUVzHwOKBsj0Zlrp9o7UW05BJUrUgVXbvJ61r2F+zoAREVSnZt5Tt3JOQs/JRFUway6AvKiQQJihLAOo6AkKiUCTR2G4kbFGiILq4hwgASZGshfdgKRCy+jbHlfDGpNF+vABIwoeTGgkil6kOH/Dg+hNKmqS8N41Y1tQn7i7RkfjMw7gMOQoZcNTKDCNGfgR0gu62ZIkDBIXmea25leCk6VnH2AGcgG4EAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVzmdAUFVV1YAAAAAAApj+2QAACcQuyA5y12P+HQ9xkG4YvVJJeqDZf4BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZaLZ4aygAAAAIyAxQV////+AAAAABnIBuBAAAAAGcgG4AAAAZXwuHPYAAAAAJwWNtUCsIlij3mTR7FLM4Pu9qzDhJrUtUxIctFWnmj84Af485oCfcURBzjS8v9xlCaHMjofeED+Ml66aUMg3GKE8PDVhr5SAP4MJU436Fr6IFOxCWwq4hIuPuRgtLh6xy3t1dAZmA1SLzhr+OAOS1cKUapaSIeOdv/Mclu2fbSsnRU72f3eNeVU1v13bHKNJ70zxX/fMj109FD2kNQf4+VnjXn0jbxUKWfH5PZBT9oXoD9C59CFRYhLKAuMLSgi1sRBH0T1SmF59vcZjsn";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **7131950295749**
/// - publish_time: **1730209108**
const VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQBLJRnF435tmWmnpCautCMOcWFhH0neObVk2iw/qtQ/jX44qUBV+Du+woo5lWLrE1ttnAPfwv9aftKy/r0pz0OdAQP25Bjy5Hx3MaOEF49sx+OrA6fxSNtBIxEkZ/wqznQAvlNE86loIz2osKoAWYeCg9FjU/8A2CmZZhcyXb4Cf+beAQSN829+7wKOw6tdMnKwtiYKdXL1yo1uP10iZ3EhU2M4cxrD0xYKA0pkb9hmhRo+zHrOY9pyTGXAsz7FjlI+gvgCAQa5MiGBgMRLFGW0fTd+bqc+isCQDbhgm/99yNkVaDt40ASST8CfH5zp4Xim5l5Yhs+/HMpeFSuTNULeDXsTO2FaAAjaPzeC8Bie6n154BaKA+45xn0lDa0epmVZs16zVCkKczSUNVG5e5VZe6N8edT+dVicoZYT9tgHJn2WDIjcpRv7AAsc0fdXE42zolp1Dhg1XVL5oe6NeTZi2Beu2ecv5FkvtCwm9dytTv6C359wJqUZLbZVaqOU9CEVbBvTzbKAm/tQAAx12qSCdkLtlJZAmhhrCvW56375q1Dy74L417r+GhDgYRqPCNWyaY7azRFfOwahxc9ECZgHj1aJg0bk395+JhTnAQ2K/IC6aRcSpPd+SfbWnfPtdJTdJFw5QCS50FbBfxxmqBTcG8E8fyYyCz5SGC8rtXgrBi+cQZe8FgW4CoLXXxC+AQ7TotPy0p9aHpwlIrXvu9B2nThByrwd4icwnOfQsUDHcG65PXWvu9nc1o5EK6SImnv+AmIu+RID2MnyTavsGEMpAA/XdQHG8mkgdWlZ1w7fg2MBs3fa0VxIlKc1DuaBdZVZEjrnB4gE15oqMZ21Bt8ji6r6J+ar/9K46EUeYC2t6CuBARDpRTI9ZZlh0MvxIbxRkuAgtRTv8oNrSz4sQJMNbhWdswTmqQQMZjtdJwGWepaAGhnEiuF/JgIr20AnDxCWbolgABGwVILVFDCHnLV54/bIdXUEiigPZvsKcDxLpOoJ722xZT1cXwXoBmwQ2lXQxGOjyj8VvgAt2kZJNbGc77+pmsqdABIFwK9Dc5BLxz+dXztA5bPMcEKkfZ18t7HPZ9BVQN7f1Cw4XcBZDSRR0MM6tqeBYvLJZhDMbt2Ax0m0+RlzQTZyAWcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo1AUFVV1YAAAAAAApl86sAACcQTdtYrFsURmdX9JeZM/nLGOdGy18BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZ8iV0qxQAAAAIvYnVX////+AAAAABnIOVUAAAAAGcg5VQAAAZ3rChYAAAAAAIykC3MCknCJZOvI3H3Ijt5NftDL77S253kTxg9ywpWvf3kzbZeQqXixw7K/fcAEWCww773jqhfS4CdRyUc38SMv+DhHywJbnUSyzFEWOTBVmVuvEtt6xWOTDMifAi8cAX0cBtZOyeIeLytWSqkMVYhtbm0gKCLnjtBEKLg/zEHSL48Ndm9VTihIpe8REto4Pf2MjlxRY6Smgw2TMZCJTCEj2869KzQsQhVSH4VmOJNJpevlYaqeFmJ7WDOC1tFWrVulGSZ/nIt63NKB+JP";

pub const WBTC_USD_ID: &str = "0xc9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33";
pub const ETH_USD_ID: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
pub const USDC_USD_ID: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
pub const SOL_USD_ID: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
pub const ATOM_USD_ID: &str = "0xb00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819";
pub const BNB_USD_ID: &str = "0x2f95862b045670cd22bee3114c39763a4a08beeb663b145d283c31d7d1101c4f";
pub const DOGE_USD_ID: &str = "0xdcef50dd0a4cd2dcc17e45df1676dcb336a11a61c69df7a0299b0150c672d25c";
pub const XRP_USD_ID: &str = "0xec5d399846a9209f3fe5881d70aae9268c94339ff9817e8d18ff19fa05eea1c8";
pub const TON_USD_ID: &str = "0x8963217838ab4cf5cadc172203c1f0b763fbaa45f346d8ee50ba994bbcac3026";
pub const SHIBA_USD_ID: &str = "0xf0d57deca57b3da2fe63a493f4c25925fdfd8edf834b20f93e1f84dbd1504d4a";

pub const PYTH_URL: &str = "https://hermes.pyth.network";

#[test]
fn oracle() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let id = PythId::from_str(WBTC_USD_ID).unwrap();
    let precision = 8;
    let btc_denom = Denom::from_str("bridge/btc").unwrap();

    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! {
                btc_denom.clone() => PriceSource::Pyth { id, precision }
            }),
            Coins::default(),
        )
        .should_succeed();

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &ExecuteMsg::FeedPrices(vec![Binary::from_str(VAA_1).unwrap()]),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                denom: btc_denom.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("69843.82159562").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("69843.82159562").unwrap()
        );

        assert_eq!(current_price.precision(), precision);

        assert_eq!(current_price.timestamp, 1730157441);
    }

    // Push an updated_price
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &ExecuteMsg::FeedPrices(vec![Binary::from_str(VAA_2).unwrap()]),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                denom: btc_denom.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("71319.50295749").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("71319.50295749").unwrap()
        );

        assert_eq!(current_price.timestamp, 1730209108);
    }

    // Push an outdated price. it should not be updated
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &ExecuteMsg::FeedPrices(vec![Binary::from_str(VAA_1).unwrap()]),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest { denom: btc_denom })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("71319.50295749").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("71319.50295749").unwrap()
        );

        assert_eq!(current_price.timestamp, 1730209108);
    }
}

#[tokio::test]
async fn double_vaas() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let mut last_btc_vaa: Option<PriceFeed> = None;
    let mut last_eth_vaa: Option<PriceFeed> = None;

    let pyth_id_btc = PythId::from_str(WBTC_USD_ID).unwrap();
    let pyth_id_eth = PythId::from_str(ETH_USD_ID).unwrap();

    let btc_denom = Denom::from_str("bridge/btc").unwrap();
    let eth_denom = Denom::from_str("bridge/eth").unwrap();

    // Register price sources
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &ExecuteMsg::RegisterPriceSources(btree_map! {
                btc_denom.clone() => PriceSource::Pyth { id: pyth_id_btc, precision: 8 },
                eth_denom.clone() => PriceSource::Pyth { id: pyth_id_eth, precision: 8 },
            }),
            Coins::default(),
        )
        .should_succeed();

    for _ in 0..5 {
        // get 2 separate vaa
        let (btc_vaas_raw, eth_vaas_raw) = tokio::try_join!(
            get_latest_vaas(&[WBTC_USD_ID]),
            get_latest_vaas(&[ETH_USD_ID])
        )
        .unwrap();

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
                contracts.oracle,
                &ExecuteMsg::FeedPrices([btc_vaas_raw, eth_vaas_raw].concat()),
                Coins::default(),
            )
            .should_succeed();

        // check btc price
        {
            let current_price = suite
                .query_wasm_smart(contracts.oracle, QueryPriceRequest {
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
                .query_wasm_smart(contracts.oracle, QueryPriceRequest {
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
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[tokio::test]
async fn multiple_vaas() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

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
                id: PythId::from_str(id).unwrap(),
                precision: 8,
            })
        })
        .collect();

    // Register price sources
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &ExecuteMsg::RegisterPriceSources(denom_price_sources),
            Coins::default(),
        )
        .should_succeed();

    let mut last_price_feeds = id_denoms
        .keys()
        .map(|id| (*id, None))
        .collect::<BTreeMap<_, Option<PriceFeed>>>();

    for _ in 0..5 {
        let vaas_raw = get_latest_vaas(id_denoms.keys()).await.unwrap();

        let vaas = vaas_raw
            .iter()
            .map(|vaa_raw| PythVaa::new(&MockApi, vaa_raw.clone().into_inner()).unwrap())
            .collect::<Vec<_>>();

        // Update last price feeds
        for vaa in vaas {
            for price_feed in vaa.unverified() {
                let last_price_feed = last_price_feeds
                    .get_mut(price_feed.id.to_string().as_str())
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
                contracts.oracle,
                &ExecuteMsg::FeedPrices(vaas_raw),
                Coins::default(),
            )
            .should_succeed();

        // Check all prices
        for (denom, last_price_feed) in &last_price_feeds {
            let denom = id_denoms.get(denom).unwrap();

            let current_price = suite
                .query_wasm_smart(contracts.oracle, QueryPriceRequest {
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
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Return JSON string of the latest VAA from Pyth network.
async fn get_latest_vaas<I>(ids: I) -> reqwest::Result<Vec<Binary>>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let ids = ids
        .into_iter()
        .map(|id| ("ids[]", id.as_ref().to_string()))
        .collect::<Vec<_>>();

    reqwest::Client::new()
        .get(format!("{PYTH_URL}/api/latest_vaas"))
        .query(&ids)
        .send()
        .await?
        .json()
        .await
}
