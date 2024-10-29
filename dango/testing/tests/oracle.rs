use {
    dango_testing::setup_test,
    dango_types::oracle::{PythId, PythVaa, QueryPriceFeedRequest},
    grug::{Binary, Coins, JsonDeExt, ResultExt},
    pyth_sdk::PriceFeed,
    std::str::FromStr,
};

#[grug::derive(Serde)]
enum RawExecuteMsg {
    UpdatePriceFeeds { data: Vec<Binary> },
}

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **6984382159562**
/// - publish_time: **1730157441**
const VAA_1:&str = "UE5BVQEAAAADuAEAAAAEDQBkMyJzGWOwAlhd3NDvcYJvct5KACRi6oi9InIE/PYqXh1z92MOXFyFPGP5y9uOpubgMIvUh/pa5aXsM/z+aaCdAALKQlwSVB5YIQ/C0NuqXqam0fAAQYUJeBe+G7rjnv7UXhHRIqNiqCvTE1ygz3zUztg07pqoYahCI7SlqI23hHizAAPG7cQdoENAUMDgYC1znnRkG8NUDS/Yzlxb3Krl/fKDUjpgKM2ZEB5HD11bCTzIhPHTI8KQxIDbyKxF6o4cwf5QAAQxrIWXQX0Bx9/lDEDfFOOqRU6LwZhFMmiDwUedUxsIvR73V/yfZKNtObHA0O9McjdTo1JibRqnbNqw6H8hw4/JAAax4DOJ/M8yxbIk88rV0n8sttzelXPuMnnJCXV2CFpwlSqYu0cQ+gmWvfjK/zJSFKHhNF0N7wzOX9J/bghUeQ8nAQgJ7BPYtJo/qowTuQfDCa4ZHIhLjC9frRQh3/UWLrxosG5xWODfYWtpDLKwfmi2gjMV4PIMUdhwZLyMDfZIqR6MAQrB/IQ438iz+1cgU+i8ij7eB5+MeUxcV0ukQhJW/0nwVCm234OqZ+ES3fNPIpWHRo4nq5ZVCdX4ZE3MF+SjZIW2AAu4DFxPpw3tokuOP6z2jNk9AFzjC/WUqlZaIx+6Se5ZeGr4chhEh2IiwChhSUJnGsKtkXHSqTuLZpXf8QZ+ZiRFAAz9XiWxbiOvw6E4+I/0JRutYrALssiRNYBah4I1QzYSU1gIAeMEHz2jvMX9lGGZMfS/uJrv1VtW9UCJMxMCUqgOAA2Hkv95hjyj6toIigG6PyEpzzoJE3ZVqI92F2kWoGSE0l/7aV/sz6jhRl8udbq/Mqu+i9wpbUZqa/ZUCFFi0NLSAQ5s3Le7hPfK1QnMOU8eWkJqiy/XL+remqBwR92Omm8FFANUVzHwOKBsj0Zlrp9o7UW05BJUrUgVXbvJ61r2F+zoAREVSnZt5Tt3JOQs/JRFUway6AvKiQQJihLAOo6AkKiUCTR2G4kbFGiILq4hwgASZGshfdgKRCy+jbHlfDGpNF+vABIwoeTGgkil6kOH/Dg+hNKmqS8N41Y1tQn7i7RkfjMw7gMOQoZcNTKDCNGfgR0gu62ZIkDBIXmea25leCk6VnH2AGcgG4EAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVzmdAUFVV1YAAAAAAApj+2QAACcQuyA5y12P+HQ9xkG4YvVJJeqDZf4BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZaLZ4aygAAAAIyAxQV////+AAAAABnIBuBAAAAAGcgG4AAAAZXwuHPYAAAAAJwWNtUCsIlij3mTR7FLM4Pu9qzDhJrUtUxIctFWnmj84Af485oCfcURBzjS8v9xlCaHMjofeED+Ml66aUMg3GKE8PDVhr5SAP4MJU436Fr6IFOxCWwq4hIuPuRgtLh6xy3t1dAZmA1SLzhr+OAOS1cKUapaSIeOdv/Mclu2fbSsnRU72f3eNeVU1v13bHKNJ70zxX/fMj109FD2kNQf4+VnjXn0jbxUKWfH5PZBT9oXoD9C59CFRYhLKAuMLSgi1sRBH0T1SmF59vcZjsn";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **7131950295749**
/// - publish_time: **1730209108**
const VAA_2:&str = "UE5BVQEAAAADuAEAAAAEDQBLJRnF435tmWmnpCautCMOcWFhH0neObVk2iw/qtQ/jX44qUBV+Du+woo5lWLrE1ttnAPfwv9aftKy/r0pz0OdAQP25Bjy5Hx3MaOEF49sx+OrA6fxSNtBIxEkZ/wqznQAvlNE86loIz2osKoAWYeCg9FjU/8A2CmZZhcyXb4Cf+beAQSN829+7wKOw6tdMnKwtiYKdXL1yo1uP10iZ3EhU2M4cxrD0xYKA0pkb9hmhRo+zHrOY9pyTGXAsz7FjlI+gvgCAQa5MiGBgMRLFGW0fTd+bqc+isCQDbhgm/99yNkVaDt40ASST8CfH5zp4Xim5l5Yhs+/HMpeFSuTNULeDXsTO2FaAAjaPzeC8Bie6n154BaKA+45xn0lDa0epmVZs16zVCkKczSUNVG5e5VZe6N8edT+dVicoZYT9tgHJn2WDIjcpRv7AAsc0fdXE42zolp1Dhg1XVL5oe6NeTZi2Beu2ecv5FkvtCwm9dytTv6C359wJqUZLbZVaqOU9CEVbBvTzbKAm/tQAAx12qSCdkLtlJZAmhhrCvW56375q1Dy74L417r+GhDgYRqPCNWyaY7azRFfOwahxc9ECZgHj1aJg0bk395+JhTnAQ2K/IC6aRcSpPd+SfbWnfPtdJTdJFw5QCS50FbBfxxmqBTcG8E8fyYyCz5SGC8rtXgrBi+cQZe8FgW4CoLXXxC+AQ7TotPy0p9aHpwlIrXvu9B2nThByrwd4icwnOfQsUDHcG65PXWvu9nc1o5EK6SImnv+AmIu+RID2MnyTavsGEMpAA/XdQHG8mkgdWlZ1w7fg2MBs3fa0VxIlKc1DuaBdZVZEjrnB4gE15oqMZ21Bt8ji6r6J+ar/9K46EUeYC2t6CuBARDpRTI9ZZlh0MvxIbxRkuAgtRTv8oNrSz4sQJMNbhWdswTmqQQMZjtdJwGWepaAGhnEiuF/JgIr20AnDxCWbolgABGwVILVFDCHnLV54/bIdXUEiigPZvsKcDxLpOoJ722xZT1cXwXoBmwQ2lXQxGOjyj8VvgAt2kZJNbGc77+pmsqdABIFwK9Dc5BLxz+dXztA5bPMcEKkfZ18t7HPZ9BVQN7f1Cw4XcBZDSRR0MM6tqeBYvLJZhDMbt2Ax0m0+RlzQTZyAWcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo1AUFVV1YAAAAAAApl86sAACcQTdtYrFsURmdX9JeZM/nLGOdGy18BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZ8iV0qxQAAAAIvYnVX////+AAAAABnIOVUAAAAAGcg5VQAAAZ3rChYAAAAAAIykC3MCknCJZOvI3H3Ijt5NftDL77S253kTxg9ywpWvf3kzbZeQqXixw7K/fcAEWCww773jqhfS4CdRyUc38SMv+DhHywJbnUSyzFEWOTBVmVuvEtt6xWOTDMifAi8cAX0cBtZOyeIeLytWSqkMVYhtbm0gKCLnjtBEKLg/zEHSL48Ndm9VTihIpe8REto4Pf2MjlxRY6Smgw2TMZCJTCEj2869KzQsQhVSH4VmOJNJpevlYaqeFmJ7WDOC1tFWrVulGSZ/nIt63NKB+JP";

pub const WBTC_USD_ID: &str = "0xc9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33";
pub const ETH_USD_ID: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
pub const PYTH_URL: &str = "https://hermes.pyth.network";

#[test]
fn oracle() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let id = PythId::from_str(WBTC_USD_ID).unwrap();

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &RawExecuteMsg::UpdatePriceFeeds {
                    data: vec![Binary::from_str(VAA_1).unwrap()],
                },
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceFeedRequest { id })
            .unwrap();

        assert_eq!(current_price.id.to_bytes(), *id);
        assert_eq!(current_price.get_price_unchecked().publish_time, 1730157441);
        assert_eq!(current_price.get_price_unchecked().price, 6984382159562);
    }

    // Push an updated_price
    {
        {
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.oracle,
                    &RawExecuteMsg::UpdatePriceFeeds {
                        data: vec![Binary::from_str(VAA_2).unwrap()],
                    },
                    Coins::default(),
                )
                .should_succeed();

            let current_price = suite
                .query_wasm_smart(contracts.oracle, QueryPriceFeedRequest { id })
                .unwrap();

            assert_eq!(current_price.id.to_bytes(), *id);
            assert_eq!(current_price.get_price_unchecked().publish_time, 1730209108);
            assert_eq!(current_price.get_price_unchecked().price, 7131950295749);
        }
    }

    // Push an outdated price. it should not be updated
    {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &RawExecuteMsg::UpdatePriceFeeds {
                    data: vec![Binary::from_str(VAA_1).unwrap()],
                },
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceFeedRequest { id })
            .unwrap();

        assert_eq!(current_price.id.to_bytes(), *id);
        assert_eq!(current_price.get_price_unchecked().publish_time, 1730209108);
        assert_eq!(current_price.get_price_unchecked().price, 7131950295749);
    }
}

#[tokio::test]
async fn multiple_vaas() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let mut last_btc_vaa: Option<PriceFeed> = None;
    let mut last_eth_vaa: Option<PriceFeed> = None;

    let pyth_id_btc = PythId::from_str(WBTC_USD_ID).unwrap();
    let pyth_id_eth = PythId::from_str(ETH_USD_ID).unwrap();

    for _ in 0..10 {
        // get 2 separate vaa
        let (btc, eth) = tokio::try_join!(
            get_latest_vaas(PYTH_URL, &[WBTC_USD_ID]),
            get_latest_vaas(PYTH_URL, &[ETH_USD_ID])
        )
        .unwrap();

        let btc_vaa = PythVaa::from_str(&btc[0]).unwrap().unverified()[0];
        let eth_vaa = PythVaa::from_str(&eth[0]).unwrap().unverified()[0];

        if let Some(last_btc_vaa) = &mut last_btc_vaa {
            if btc_vaa.get_price_unchecked().publish_time
                > last_btc_vaa.get_price_unchecked().publish_time
            {
                last_btc_vaa.clone_from(&btc_vaa);
            }
        } else {
            last_btc_vaa = Some(btc_vaa);
        }

        if let Some(last_eth_vaa) = &mut last_eth_vaa {
            if eth_vaa.get_price_unchecked().publish_time
                > last_eth_vaa.get_price_unchecked().publish_time
            {
                last_eth_vaa.clone_from(&eth_vaa);
            }
        } else {
            last_eth_vaa = Some(eth_vaa);
        }

        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &RawExecuteMsg::UpdatePriceFeeds {
                    data: vec![
                        Binary::from_str(&btc[0]).unwrap(),
                        Binary::from_str(&eth[0]).unwrap(),
                    ],
                },
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceFeedRequest { id: pyth_id_btc })
            .unwrap();

        assert_eq!(current_price.id.to_bytes(), *pyth_id_btc);
        assert_eq!(
            current_price.get_price_unchecked().publish_time,
            last_btc_vaa.unwrap().get_price_unchecked().publish_time
        );
        assert_eq!(
            current_price.get_price_unchecked().price,
            last_btc_vaa.unwrap().get_price_unchecked().price
        );

        let current_price = suite
            .query_wasm_smart(contracts.oracle, QueryPriceFeedRequest { id: pyth_id_eth })
            .unwrap();

        assert_eq!(current_price.id.to_bytes(), *pyth_id_eth);
        assert_eq!(
            current_price.get_price_unchecked().publish_time,
            last_eth_vaa.unwrap().get_price_unchecked().publish_time
        );
        assert_eq!(
            current_price.get_price_unchecked().price,
            last_eth_vaa.unwrap().get_price_unchecked().price
        );
    }
}

pub async fn get_latest_vaas(url: &str, ids: &[&str]) -> anyhow::Result<Vec<String>> {
    let url = format!("{url}/api/latest_vaas");
    let ids = ids.iter().map(|id| ("ids[]", id)).collect::<Vec<_>>();
    let client = reqwest::Client::new();
    let response = client.get(url).query(&ids).send().await?;
    Ok(response.text().await?.deserialize_json()?)
}
