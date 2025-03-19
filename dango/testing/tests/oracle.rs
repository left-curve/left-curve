use {
    dango_oracle::PRICES,
    dango_testing::{TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        constants::{
            ATOM_DENOM, BNB_DENOM, BTC_DENOM, DOGE_DENOM, ETH_DENOM, SHIB_DENOM, SOL_DENOM,
            USDC_DENOM, WBTC_DENOM, XRP_DENOM,
        },
        oracle::{ExecuteMsg, PrecisionlessPrice, Price, QueryPriceRequest},
    },
    grug::{
        Addr, Binary, Coins, Inner, MockApi, NonEmpty, QuerierExt, ResultExt, StorageQuerier,
        Udec128, btree_map,
    },
    grug_app::NaiveProposalPreparer,
    pyth_client::{PythClientCache, PythClientTrait},
    pyth_types::{
        ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, PYTH_URL, PythId, PythVaa,
        SHIB_USD_ID, SOL_USD_ID, USDC_USD_ID, XRP_USD_ID,
    },
    std::{cmp::Ordering, collections::BTreeMap, str::FromStr, thread, time::Duration},
};

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **6864578657006**
/// - publish_time: **1730804420**
const VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQBnC+7yOL2qsxrpxHzhTnaruVWTSfjBRIF7sk1bJUZzj3s7wZyytPTHtoxXFQaFFSCgVpCXeLdeHuN3ZM2LOvQMAAMOXaxpZYUuwjEhbN8yP3wfgSDdaFgiS0Abr1Hyf29BX1sYEEH82xUVspIdEv7DBves+XjKJWWnZ51De4KMmDqgAQR9ExeR/D3QbvfFarB73jLQ+QKGS0tb50229RyjKCHv2VbRJL5go04kePmSqLjqjhBn/IBx2Rr1W16DF9fKV2h+AAaPsmegjpPIfPIDZwqMcgvNfXqG77+8RYSH95azsCTMEFOaQVtJGJbjQUWdSrlqXukLgxIxf6yKdzp7sOBNFFVdAAhXAe1EFhONyQgWDnViECw7DbvmwNtjJ2xM/DslvZ2RJVA46pZ5St6IKyK2Ucqq/0Hu2nC1CEB39Rtcvu0Sm6DCAQpyh2KzwK+i9CtzyZNYfRFn+esWmnSHpoZrBYLgxayqtRIiTPetE3hudyHUxm4xk7CfcBrRD8uThsny1YHeiQpiAQtcR4XqjxUWHNLXsMaqaF3B/pskIjxVjWEiDkJCIpqoJFn8tktkDh00XREbZ68SUhUQQ1/S6icJLUIQt2Rf4cy5AAwJVyMi0NmjVs0X5NYzwO1Uk6Yfx96HQtibi9gPiCR4gXTW0udFzqvQ2u2xiiXonGjmaRMW86hm/6kx08d341PTAQ2ypzyZJiPhPZAo4I2IJtdjkq72uyR4lL1kqaIGupLxtCq36i1tD61Yjt3HRruBuVvHqjC60xDvWIVQL6UAHAu9AQ7wH5SeZ1ra473yrfVGIEtuGSh0iITJ3Tnzh+4IJMdnvjFARCrxHLmne50gjYcG+CQYSHl/TJ+fElFtiDx43ouGABCT8qRJAJYpusR2A1mGXDX/oBSq0NoaKKr7u4c8zLDsLWUudBRRVkDS4281f+GuQupa5eRPKdDHXt40lFY5V+FWABEcD+ka2buu8h4ZAK9gWcOhe9Ms0COktqchnwS3oZV7lXXcZM1K+LKc+gKshOln7r3JC1UrkcjJa6gy9v5Ka9YTARIYLaAd0TkttKtK9hoALKRTkEqpqgtvBLqJA9qW1UDYoAZksJo2X0th7lFdIZJQsCIkDxqedbuS1H7EQ7Im6XUHAGcp+sQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFcADhAUFVV1YAAAAAAAp8yzcAACcQXNrQIBXy4Cs6ul3jv4wlMishtwkBAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAY+SMW67gAAAAI/1aJY////+AAAAABnKfrEAAAAAGcp+sQAAAY+IAqHoAAAAAIKJkDECnJa8p4N3HJckOG0/XBHQ4HSCFfFvzHVwHvYJ9V5NPKGlOHwUp0GbOXWbNIMhSmoX+hk8FUMlP6NlHHbf8S2YxVixm+nMOOrhtH9+3bMQQh26XE6/E5UIoNgScjtRRQ32qtHxrU1ezhAhHmTAAD07E8S/ACc8F8xjDAZgLgjSFLHptczUSe1wR5IrrbZQRQhERagNdCcBUp8S5wl7VAQBPqprw5ZZ6dvI0y/P8UaldqRoa8eN47BbGvH/12oNzfcUiLjHCFciAwc";

/// - id: **c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33**
/// - price: **7131950295749**
/// - publish_time: **1730209108**
const VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQBLJRnF435tmWmnpCautCMOcWFhH0neObVk2iw/qtQ/jX44qUBV+Du+woo5lWLrE1ttnAPfwv9aftKy/r0pz0OdAQP25Bjy5Hx3MaOEF49sx+OrA6fxSNtBIxEkZ/wqznQAvlNE86loIz2osKoAWYeCg9FjU/8A2CmZZhcyXb4Cf+beAQSN829+7wKOw6tdMnKwtiYKdXL1yo1uP10iZ3EhU2M4cxrD0xYKA0pkb9hmhRo+zHrOY9pyTGXAsz7FjlI+gvgCAQa5MiGBgMRLFGW0fTd+bqc+isCQDbhgm/99yNkVaDt40ASST8CfH5zp4Xim5l5Yhs+/HMpeFSuTNULeDXsTO2FaAAjaPzeC8Bie6n154BaKA+45xn0lDa0epmVZs16zVCkKczSUNVG5e5VZe6N8edT+dVicoZYT9tgHJn2WDIjcpRv7AAsc0fdXE42zolp1Dhg1XVL5oe6NeTZi2Beu2ecv5FkvtCwm9dytTv6C359wJqUZLbZVaqOU9CEVbBvTzbKAm/tQAAx12qSCdkLtlJZAmhhrCvW56375q1Dy74L417r+GhDgYRqPCNWyaY7azRFfOwahxc9ECZgHj1aJg0bk395+JhTnAQ2K/IC6aRcSpPd+SfbWnfPtdJTdJFw5QCS50FbBfxxmqBTcG8E8fyYyCz5SGC8rtXgrBi+cQZe8FgW4CoLXXxC+AQ7TotPy0p9aHpwlIrXvu9B2nThByrwd4icwnOfQsUDHcG65PXWvu9nc1o5EK6SImnv+AmIu+RID2MnyTavsGEMpAA/XdQHG8mkgdWlZ1w7fg2MBs3fa0VxIlKc1DuaBdZVZEjrnB4gE15oqMZ21Bt8ji6r6J+ar/9K46EUeYC2t6CuBARDpRTI9ZZlh0MvxIbxRkuAgtRTv8oNrSz4sQJMNbhWdswTmqQQMZjtdJwGWepaAGhnEiuF/JgIr20AnDxCWbolgABGwVILVFDCHnLV54/bIdXUEiigPZvsKcDxLpOoJ722xZT1cXwXoBmwQ2lXQxGOjyj8VvgAt2kZJNbGc77+pmsqdABIFwK9Dc5BLxz+dXztA5bPMcEKkfZ18t7HPZ9BVQN7f1Cw4XcBZDSRR0MM6tqeBYvLJZhDMbt2Ax0m0+RlzQTZyAWcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo1AUFVV1YAAAAAAApl86sAACcQTdtYrFsURmdX9JeZM/nLGOdGy18BAFUAydiwdaXGkwM2WuI2M9TghRmb9cUgo7kP7RMioDQv/DMAAAZ8iV0qxQAAAAIvYnVX////+AAAAABnIOVUAAAAAGcg5VQAAAZ3rChYAAAAAAIykC3MCknCJZOvI3H3Ijt5NftDL77S253kTxg9ywpWvf3kzbZeQqXixw7K/fcAEWCww773jqhfS4CdRyUc38SMv+DhHywJbnUSyzFEWOTBVmVuvEtt6xWOTDMifAi8cAX0cBtZOyeIeLytWSqkMVYhtbm0gKCLnjtBEKLg/zEHSL48Ndm9VTihIpe8REto4Pf2MjlxRY6Smgw2TMZCJTCEj2869KzQsQhVSH4VmOJNJpevlYaqeFmJ7WDOC1tFWrVulGSZ/nIt63NKB+JP";

/// BTC_USD_ID
/// This data have same publish_time but different price and different sequence,
/// to test the update logic inside the oracle.
const OLD_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQC1PrJvEJdrAATLXCaO9KPKy8jg60pFB+/dG5WeLIjHyw8uUxNF/UQ54R2oBxGF97NHwHRZtF0/Sra0XBijMAuOAALeIiPvZN3bcnWRMAqSwZCUYN/SZ79xHQhJQ07rM85SHVzUYMo5LCiuV1h0O9tvA5kmyABfyV2a3Y7eQZoroInKAAN6LJGqKqSo6ZaSCMVBZ+6tQ+Tr4+IgUrzlxIg/kCfZoVZbgTKiBmt1iDhcLkwC+bWOxHpHloqISFpPJOXDuCBGAAT9vCGgUCSflqT/muUoQ5qFtC14RaTWjVw6CKINncZntA2cN/yJw6A+Xs5U/DU+bmN35PxGzD6q7FjFb5X9qbqhAQZ/wwZDiZCdVa3ra9I6CQ+e6SSulXxUTj/bZFd+5ijKpR0CCOIa+aqYIUyACSGjh2HZdo4b74kGEWmBU84qHkszAAg1p30sJ8vyw9W38JrZd+OnOBfxiii0Y12+xZWbzdWrfibFvYiuJkxGdWm+G5pUie+hrgC8rv/Gr37Vup6U2JOIAQvMUewdWzGDQ4RdJlG9JE46eZwY6+tXVvcPN3HWGpialyJUtpbFzU2A+lYiP1U+me1HX+LsUXLLgtxCyUYfSvY1AQwBEE2estPGKI3D55X+U5lazI5vLEbqN+3Ek9HfChwr0VDHpvtOcxvxFAuC7A05KkgtJ2DlDGqQzjSCMIqLunhLAQ0ImWBmzfbh71buxjRd3pcx2u0vBr22b3hZmdvgosd4kQUYiXmQqqSdYEUAsYsI8lS6IY5tKmmt5Ne3Db1cfkLiAQ5fwFgtmfxP9vhNzQH9wSDy3uGWeTY8QT6Nth4WorwqPwHIoLJ5GoPQmhmQ+Lamm1iRZkHkeiWMcJFmrXvjMs60AQ9t6nrW9X8iPHXhJidZkc30tXZTbte6HFu4wd+d2r9P/DrmqeuX0Z1ImR1h3PXhO8+llZcYymepesPG7oYYdNebARDK6B2HQ+YqZy39uLSvv9Ixc5PAFVE3Es69ZOIIzyNqPhO3XnlPYBPpBuC9bzXhxacenfk9++YnO7wsVTAjarfmARG5FUbdJ3/pnJC1hfeXgDg9ZWu1vhXSJAPgOpJLlIhvaTgZk7JWSMBaOALtD8yaea6t16LM4XDzdeWrUdGFgBLAAGfUI2QAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAHG0BzAUFVV1YAAAAAAAwoR/4AACcQHJX9oR7eAJ3kDAJmGcmHXSG1M8QBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAeXyVYyXAAAAAC5aSAk////+AAAAABn1CNkAAAAAGfUI2MAAAeRixECAAAAAAC1MsVIDGVIb4bQKzzTbGYwasa37r9mN/OGqfaDpf2gg0Je0EyRbd7dDLTzEhSNqJBsntjW9sMHHu0TOkqpJI7ygjPSNAO2qbAU4lj8KAUFRZ1hP4SISP2UX0V5NrhALGOCWnTQErcCdy6Gm6orqVJhq/jLQ0Jljvzq9N38b/tmDKeyzJl3MufHg80fBOppdw5W4zJg54UZl1gqb6JGeE5vMPDoze5fZ2UbOWtcwS/HfHEq0JvH3z9zbS5Kt8brXfQbODqiWLmHQPIf+w+xe147+bsQ/2jd3Kigq7h6O1wbrI8xHT1QhxhdBItjPhUO7U6SCAW+5A==";
const NEW_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQDJVzkTXJ/xX0Z6yCF9CeCDwGIskGvr0dgHi5DHq+ZiWHS/fqNgstRz7U0k50dTMJak7JOmQePRMWa0abZ4OQv8AQJEfiU38lpjC5mwmbFkdiNU7M3FxdcmEyXqCWHclPbJEDyfgDYamXDQfIbsEzjtf959ZGhbqjmqWQ86zkOcU6pEAANeXPg2UPUQvAiP1l6HDQhkcgu8VHiNJorAWboTw+LwtwotgE4JuvZFCCcNI/gbdFND+Cf7kDulUMd+SNXxlcWLAASXfzh0K3S79739gQXlJ7lK5jdv33I4U70Ma+5COdN5MkhPP+wcYc4ZNCNkC6GhwMknxmbDGcxUVlge9CmdmBfLAAYQ/2bhKpo4fdPHExfK9gl74JGIDkkJMv8D9OXjVQBVEnutA2ucSYtFFmdPoFW8Od4k5vrO0XiVnpajYdhtA/DNAQh0nCn4dmbGvp5uQfRTQ+b7IivCtOSusaIIlN0ippA1E0pxERsyngqICHH4NJazflsgqWbo6KxzuTf7UkMEEF9gAQrhy+VQDRgIjUns3BLueS3DstsrBkKaglXFtqjM+GSU5l+8vqGOSl2gLGU3KCOLYt5FKzrr9BenG+Fr2DKfHHuQAAwPN3YKI48TY7o0d7dKws5Uv9IG2ILMF7+SZU1gd4/ydjSpYADn6msqab1d9+q+tkaW5DPi1f+p93Wh3Kr8eYygAQ3LCDbzf7z/nsDesRUmEyv944SYqa+AHOEWDAMhNfu6qgM8DkvJROK6vcgF9+SrfJN/0W+l3RCR1GfRyTMSMFLtAA6n1JaDtwLbcmgmCfnDIzeP2HF8LktHJiHNxSbAtFLFjneL+8fz69eSBqEX8Szm7vWWTkuVzLfVPoqfSzDsFdsNAA8ds4VTQTUvVMTRr6OUDkYZmFFHf9S6Z9Aan94Fr08CT0k7jv80Csk6sfa55Zd5e+/llRR4YOVtC0SXfKtmhmnIABDIgEd/lNqC8c8s5A9sJwklvjbdDheNroZ1A3hCBq017CqGHjweC1Cd3byISTVpV+Rpl4Hg8VK3R+PgaNwRwT1vARHeO2usqEiKywPA+GgMGfQSYOkYXQkzZ80CB0i37KHgIlbqrDaBYzE/WD/0ugcetCE2saF2quy2+eVddIeazgfWAGfUI2QAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAHG0B0AUFVV1YAAAAAAAwoR/8AACcQ6c3sPfMBZYx5zE2zbC46nih06gIBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAeXzlFGBwAAAAC7lRp5////+AAAAABn1CNkAAAAAGfUI2QAAAeRi0Zp4AAAAAC1MvW4DEFMNDUv+h3foglPFliT76Pd9cXmhIjNtClQyN06tX+QlUXpaO/a8IRr/9eZMk1j8T/WdU1ITKEvwDSbkpCWAsqvPeRstBPAtUZvVYk9BkTVCcES9uDSDc9jFPlJBp23zY1+TZb1wC8bZjn+qhDAM7kytlpRBMLGLuJWmqGEOheF82gcxQNlG+V+sh9lxwHJekKJiW0Ni/PxM53JqM/o6M+MtkYZIgLRL8nrDSGG4DxSJ/iIvl9fgTfa8tmWGI+pw96LtQw4GywuNjFAKbdHpaHNSiV/uNB22f7MoV1I8rscwQLmrRbopdKLYQRHGk3apg==";

fn setup_oracle_test() -> (TestSuite<NaiveProposalPreparer>, TestAccounts, Addr) {
    let (suite, accounts, _, contracts) = setup_test_naive();
    (suite, accounts, contracts.oracle)
}

#[test]
fn oracle() {
    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    // Push price
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_2).unwrap(),
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest {
                denom: WBTC_DENOM.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec128::from_str("71319.50295749").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec128::from_str("71110.59200000").unwrap()
        );

        assert_eq!(current_price.precision(), 8);

        assert_eq!(current_price.timestamp, 1730209108);
    }

    // Push an updated_price
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_1).unwrap(),
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest {
                denom: WBTC_DENOM.clone(),
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

        assert_eq!(current_price.timestamp, 1730804420);
    }

    // Push an outdated price. it should not be updated
    {
        suite
            .execute(
                &mut accounts.owner,
                oracle,
                &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![
                    Binary::from_str(VAA_2).unwrap(),
                ])),
                Coins::default(),
            )
            .should_succeed();

        let current_price = suite
            .query_wasm_smart(oracle, QueryPriceRequest {
                denom: WBTC_DENOM.clone(),
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

        assert_eq!(current_price.timestamp, 1730804420);
    }
}

#[test]
fn multiple_vaas() {
    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    let pyth_client = PythClientCache::new(PYTH_URL).unwrap();

    let id_denoms = btree_map! {
        ATOM_USD_ID => ATOM_DENOM.clone(),
        BNB_USD_ID  => BNB_DENOM.clone(),
        DOGE_USD_ID => DOGE_DENOM.clone(),
        ETH_USD_ID  => ETH_DENOM.clone(),
        SHIB_USD_ID => SHIB_DENOM.clone(),
        SOL_USD_ID  => SOL_DENOM.clone(),
        USDC_USD_ID => USDC_DENOM.clone(),
        BTC_USD_ID  => BTC_DENOM.clone(),
        XRP_USD_ID  => XRP_DENOM.clone(),
    };

    let ids = NonEmpty::new_unchecked(id_denoms.keys().cloned().collect::<Vec<_>>());

    let mut last_prices_data = id_denoms
        .keys()
        .map(|id| (*id, None))
        .collect::<BTreeMap<_, Option<(PrecisionlessPrice, u64)>>>();

    for _ in 0..5 {
        let vaas_raw = pyth_client.get_latest_vaas(ids.clone()).unwrap();

        let vaas = vaas_raw
            .iter()
            .map(|vaa_raw| PythVaa::new(&MockApi, vaa_raw.clone().into_inner()).unwrap())
            .collect::<Vec<_>>();

        // Update last price feeds
        for vaa in vaas {
            let new_sequence = vaa.wormhole_vaa.sequence;

            for price_feed in vaa.unverified() {
                let last_price_data = last_prices_data
                    .get_mut(&PythId::from_str(&price_feed.id.to_string()).unwrap())
                    .unwrap();

                let new_price = Price::try_from(price_feed).unwrap();

                match last_price_data {
                    Some((last_price, last_sequence)) => {
                        match last_price.timestamp.cmp(&new_price.timestamp) {
                            Ordering::Less => *last_price_data = Some((new_price, new_sequence)),
                            Ordering::Equal => {
                                if *last_sequence < new_sequence {
                                    *last_price_data = Some((new_price, new_sequence));
                                }
                            },
                            Ordering::Greater => continue,
                        }
                    },
                    None => *last_price_data = Some((new_price, new_sequence)),
                }
            }
        }

        // Check if all prices has been fetched
        for v in last_prices_data.values() {
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
        for (denom, last_price_feed) in &last_prices_data {
            let (last_price, _) = last_price_feed.clone().unwrap();

            let denom = id_denoms.get(denom).unwrap();

            let current_price = suite
                .query_wasm_smart(oracle, QueryPriceRequest {
                    denom: denom.clone(),
                })
                .unwrap();

            assert_eq!(current_price.timestamp, last_price.timestamp);
            assert_eq!(current_price.humanized_price, last_price.humanized_price);
            assert_eq!(current_price.humanized_ema, last_price.humanized_ema);
        }

        // sleep for 1 second
        thread::sleep(Duration::from_millis(500));
    }
}

// This test focus on testing the update logic inside the oracle contract when the
// publish_time of new data are the same of the stored ones. The case for different
// publish_time is already tested in the others tests.
#[test]
fn test_sequence() {
    let id = BTC_USD_ID;

    let old_vaa = Binary::from_str(OLD_VAA).unwrap();
    let old_pyth_vaa = PythVaa::new(&MockApi, old_vaa.clone().into_inner()).unwrap();
    let old_price = Price::try_from(old_pyth_vaa.clone().unverified()[0]).unwrap();

    let new_vaa = Binary::from_str(NEW_VAA).unwrap();
    let new_pyth_vaa = PythVaa::new(&MockApi, new_vaa.clone().into_inner()).unwrap();
    let new_price = Price::try_from(new_pyth_vaa.clone().unverified()[0]).unwrap();

    // Assert that the publish_time of old and new vaa are the same.
    assert_eq!(old_price.timestamp, new_price.timestamp);

    // Assert the price are different.
    assert_ne!(old_price.humanized_price, new_price.humanized_price);

    // Assert the new sequence is greater than old sequence.
    assert!(new_pyth_vaa.wormhole_vaa.sequence > old_pyth_vaa.wormhole_vaa.sequence);

    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    // Upload the old price.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![old_vaa.clone()])),
            Coins::default(),
        )
        .should_succeed();

    let (price, sequence) = suite
        .query_wasm_path(oracle, &PRICES.path(id))
        .should_succeed();

    // Data should be updated.
    assert_eq!(old_price.timestamp, price.timestamp);
    assert_eq!(old_price.humanized_price, price.humanized_price);
    assert_eq!(old_pyth_vaa.wormhole_vaa.sequence, sequence);

    // Upload the new price.
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![new_vaa])),
            Coins::default(),
        )
        .should_succeed();

    let (price, sequence) = suite
        .query_wasm_path(oracle, &PRICES.path(id))
        .should_succeed();

    // Data should be updated.
    assert_eq!(new_price.timestamp, price.timestamp);
    assert_eq!(new_price.humanized_price, price.humanized_price);
    assert_eq!(new_pyth_vaa.wormhole_vaa.sequence, sequence);

    // Try to upload the old prices (should not modify the data inside oracle).
    suite
        .execute(
            &mut accounts.owner,
            oracle,
            &ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![old_vaa.clone()])),
            Coins::default(),
        )
        .should_succeed();

    let (price, sequence) = suite
        .query_wasm_path(oracle, &PRICES.path(id))
        .should_succeed();

    // Data should not be updated.
    assert_eq!(new_price.timestamp, price.timestamp);
    assert_eq!(new_price.humanized_price, price.humanized_price);
    assert_eq!(new_pyth_vaa.wormhole_vaa.sequence, sequence);
}
