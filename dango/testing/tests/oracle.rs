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
/// - price: **6864578657006**
/// - publish_time: **1730804420**
const VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQBnC+7yOL2qsxrpxHzhTnaruVWTSfjBRIF7sk1bJUZzj3s7wZyytPTHtoxXFQaFFSCgVpCXeLdeHuN3ZM2LOvQMAAMOXaxpZYUuwjEhbN8yP3wfgSDdaFgiS0Abr1Hyf29BX1sYEEH82xUVspIdEv7DBves+XjKJWWnZ51De4KMmDqgAQR9ExeR/D3QbvfFarB73jLQ+QKGS0tb50229RyjKCHv2VbRJL5go04kePmSqLjqjhBn/IBx2Rr1W16DF9fKV2h+AAaPsmegjpPIfPIDZwqMcgvNfXqG77+8RYSH95azsCTMEFOaQVtJGJbjQUWdSrlqXukLgxIxf6yKdzp7sOBNFFVdAAhXAe1EFhONyQgWDnViECw7DbvmwNtjJ2xM/DslvZ2RJVA46pZ5St6IKyK2Ucqq/0Hu2nC1CEB39Rtcvu0Sm6DCAQpyh2KzwK+i9CtzyZNYfRFn+esWmnSHpoZrBYLgxayqtRIiTPetE3hudyHUxm4xk7CfcBrRD8uThsny1YHeiQpiAQtcR4XqjxUWHNLXsMaqaF3B/pskIjxVjWEiDkJCIpqoJFn8tktkDh00XREbZ68SUhUQQ1/S6icJLUIQt2Rf4cy5AAwJVyMi0NmjVs0X5NYzwO1Uk6Yfx96HQtibi9gPiCR4gXTW0udFzqvQ2u2xiiXonGjmaRMW86hm/6kx08d341PTAQ2ypzyZJiPhPZAo4I2IJtdjkq72uyR4lL1kqaIGupLxtCq36i1tD61Yjt3HRruBuVvHqjC60xDvWIVQL6UAHAu9AQ7wH5SeZ1ra473yrfVGIEtuGSh0iITJ3Tnzh+4IJMdnvjFARCrxHLmne50gjYcG+CQYSHl/TJ+fElFtiDx43ouGABCT8qRJAJYpusR2A1mGXDX/oBSq0NoaKKr7u4c8zLDsLWUudBRRVkDS4281f+GuQupa5eRPKdDHXt40lFY5V+FWABEcD+ka2buu8h4ZAK9gWcOhe9Ms0COktqchnwS3oZV7lXXcZM1K+LKc+gKshOln7r3JC1UrkcjJa6gy9v5Ka9YTARIYLaAd0TkttKtK9hoALKRTkEqpqgtvBLqJA9qW1UDYoAZksJo2X0th7lFdIZJQsCIkDxqedbuS1H7EQ7Im6XUHAGcp+sQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFcADhAUFVV1YAAAAAAAp8yzcAACcQXNrQIBXy4Cs6ul3jv4wlMishtwkBAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAY+SMW67gAAAAI/1aJY////+AAAAABnKfrEAAAAAGcp+sQAAAY+IAqHoAAAAAIKJkDECnJa8p4N3HJckOG0/XBHQ4HSCFfFvzHVwHvYJ9V5NPKGlOHwUp0GbOXWbNIMhSmoX+hk8FUMlP6NlHHbf8S2YxVixm+nMOOrhtH9+3bMQQh26XE6/E5UIoNgScjtRRQ32qtHxrU1ezhAhHmTAAD07E8S/ACc8F8xjDAZgLgjSFLHptczUSe1wR5IrrbZQRQhERagNdCcBUp8S5wl7VAQBPqprw5ZZ6dvI0y/P8UaldqRoa8eN47BbGvH/12oNzfcUiLjHCFciAwc";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **6869052261900**
/// - publish_time: **1730804507**
const VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQCQYvoq/i1kvPDl/g24s3eN70PnvYYG+H4zRSoaSBmkISoqWnfYftOOfyjk25frsB9T/Lk/KzEbagR61UejhabGAAK2hLc/1zQXAxTKm9Rh5vc198i6w+vfR4JhkwZkHa/gVz9PN6L3mTMlVghXbNJbPNT1mUiU9otZJH8CIHIX7cawAAQE0GzaS3e4wRCXGkrNpL/+TPwcsQNEvXxB/v7dJ+zShDjTpw1jfxBzSMfiWGKmDZboqvZPbYseJB+roLE96wXEAAajPi0ZH9X9ONP8BWuWtJ5xiM/pWgEcDAUCcGyq+6xwjBXsXgORLyUl5cKuKHrccWY0yr4Wva6h37anehLf0WI3AAgV2OjwHcensfE6KsjAr3xr9r9Hgkc42eDms+Wrts5kRmS9569q/roQtz/sS6m7nIb+qK7YBn5eLEilgeO4/4kOAAr0d+H4B9BussFMyDiLW8TKi1GLqy+9zhoSevseujd2pzi+qwGVdaoh1Nq1BXr2dO668mGw660EirxTdVy6PpBwAQsq8igSZ5wsKyJpumVqRxq+mH3oXRFc+aTH2ZY203/+cX+9AvEHXS2v8obYrR8P1JDNUj77msWNFl1Tmc+TNkBxAAxylGnWTn/mdkSJLjDGV5xpOYK5PeP8kLIGfqATUjIrKyP65tc2/hMVWOETqOZ2xfUVdI2LXAiK2akp/sY9kTo8AA1Wo5/+In5VxWc10F+pg7x8nOlUTeg3sSahfDrPlSf47QtN7s+b/c8qbd0aInHfHVYQ3HOq88mTCr96E9VewHzrAQ7chSAmTFqDIr1Ry7CDvUyW87TXABL7az0+XX29FlTkJhkBvy+qzipM+MbdjvntfXV564Yxv1AIFM+fjSdadPkxAA8ASYrpophJk9Lf8X6s/sw8GtgJRFloTA0VUuk+Bukj4FLgT2h8vr3vrv1FWggeXdE0Cug9PDWVpXuPh5raODN8ABCToVZRJVDhgiETJtqOoJmWMDLowY86d0RwtSrNZHCNqQaZR4E8jXXfCAjcd42EfbzgnzByemEy/VtDUR3Hx/vaAREMD4IP4NlsKqSpLpH2U43BNTcWVvKIjzQ1MK0BWfcz0F1i3F3VV0LtKZtwkssPl/yWjuZwdBYv2y/PsgTHd4XlAWcp+xsAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFcAG7AUFVV1YAAAAAAAp8zBEAACcQ5tVCzh23HGjssSqB9Kw6KEcY2ugBAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAY/U2uGDAAAAAJxvtvo////+AAAAABnKfsbAAAAAGcp+xsAAAY+I4u/IAAAAAIK86WgCkE5CRNkG97vEyx4mg+hqX1ykyWZz3L56uKIflDOjeTwUtcx40Q4DccFzkX7hnyZsqzj+I4bgVtwU2I+CVMD+4/+u5hTMl3J9FOOfEaTyvF6cXFF9HMjZhG7epnmaH8tQjqaZlSYdzJUtdO0qWdPlObMmN2zqmraViNt/MHZHVRFo1+USmNBnfUclxikv35VKCdC6LaBB5MyEeR2Pg/POA30I4YMQnUFgvkTMGy6RfJJZoaqr+Rn/nYZP6fdrSVKHbyGRTBAT7oo";

pub const WBTC_USD_ID: &str = "0xc9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33";
pub const ETH_USD_ID: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
pub const USDC_USD_ID: &str = "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";
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
            Udec128::from_str("68690.52261900").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("68639.54100000").unwrap()
        );

        assert_eq!(current_price.timestamp, 1730804507);
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
