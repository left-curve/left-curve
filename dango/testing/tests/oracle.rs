use {
    dango_oracle::PRICES,
    dango_testing::{TestAccounts, TestSuite, setup_test_naive},
    dango_types::{
        constants::{atom, bnb, btc, doge, eth, sol, usdc, xrp},
        oracle::{ExecuteMsg, PrecisionlessPrice, Price, QueryPriceRequest},
    },
    grug::{
        Addr, Binary, Coins, Inner, MockApi, NonEmpty, QuerierExt, ResultExt, StorageQuerier,
        Timestamp, Udec256, btree_map,
    },
    grug_app::NaiveProposalPreparer,
    pyth_client::{PythClientCache, PythClientTrait},
    pyth_types::{
        PythId, PythVaa,
        constants::{
            ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, PYTH_URL, SOL_USD_ID,
            USDC_USD_ID, XRP_USD_ID,
        },
    },
    std::{cmp::Ordering, collections::BTreeMap, str::FromStr, thread, time::Duration},
};

// Get Pyth data using the API:
// - https://hermes.pyth.network/api/get_price_feed?id=[price_feed_id]&publish_time=[publish_time_in_unix_timestamp]
// - https://hermes.pyth.network/api/get_vaa?id=[price_feed_id]&publish_time=[publish_time_in_unix_timestamp]

/// - asset: BTC-USD
/// - id: `0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43`
/// - price: `6874484759622`
/// - publish_time: `1730804420` (newer than `VAA_2`)
const VAA_1: &str = "UE5BVQEAAAADuAEAAAAEDQCcOi+hJ/yXuwFGhbNZuHWJOkYjF5s9LSRt0Y5hxb4NsxhlI7ACopeSsDE2gzNQx7B9ttrjtlVoojyNojkZW6gMAANzL+bMmo1R177GxmrK5+pdk5p8WJ0Ma7tHzz6/0Qpxw0i3Ompvj7+RulfIOnXKuqOcBCTSwTlk/ORgtOExIlUxAAQanXTR9nmps1gdBsXypwpI9+MJHeRaXRFRCTZAXZ3yJwL7CCYMbLc7gEVjKHlcza9nW4WbQzD9gizNz6y38kEXAAZmh7DytS72sivVU4Yh6+Oehithu7r6l5IPK4teinQ1gg9F89+SlKn9Y66ua8RfsFg0Aq97lLZK3xXEMms+7KFGAQggk74AariGC6+rwJ3xtVudtK8sG9X6Nb1u5YIO54/CckbdStm6A70VwYxx0ipUF/sKv+jVM4qpOUf9RzBpK7x7AQpiPs1IBwNQdsGIWClk9Hl9wanAP6j72KOB6klxxbMOAXrJTq+nqVEH3Q+pLEF023bpN00T+h9Uv6RS6orZRR8WAAup2fZAibgwJFDON39VXm3miz/biMyObbAtg4KXlGJJbkeHsdCe9WmCewP6OHSRd8U3zqgO895/e8nKQCwAI+2+AQ34VzNIFYz8BrNjPHfaPAOJDJD4uIJtcc3cqIPGBlUtDQIpZ8t04wM52Q9HSvlccMetPUAd+33xGhHVwOGlfrf2AQ6A3RBpZncC/cNA+xIzRXkeuv5Clc7Mi7RVOvEU9qacHAR03/+YPIvfCoqom62gvZneouvjI4t024WtUMYk/SS1AQ+sNZm0d9CiUUv3/8wQA2You7oUqhGyKhCeN8LHJG5q5wj5oA80zQWGyJIFPb1m9Z1zFgLpD8fbnpx+GL8zcKvQABBey7usumroT11PS4qojzgOX8wXQrlGq9h3ArytYSra4AD68UcVlogUvm8vOEaPOAUOolSiDpQGQESoeQU5aV7hABGW+0LI5vWl4IKfKp7BCeRwNzYDpjakRnaDst2EqCdYGk5J2M2KNc0M7YFPpMI3KipHJaKqVxpR46mYA/oBcmRZARLeIcwyU/IsTBW32MJfvjV1NPboTHvrkLgPjAwzbnCs2wClWGA5HCRz8JBo3KNId0Ss1Qf0O4tjsE4q9VOR3/pDAGcp+sQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFcADfAUFVV1YAAAAAAAp8yzUAACcQgoeGpiDCic4NIzrM5kpVCCwrHscBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAZAlzjcRgAAAAC6bl+r////+AAAAABnKfrEAAAAAGcp+sMAAAZAecSVQAAAAADqMpj0Cv023fUYzNkP+nDFgJnmLkV6UfLSe4N4x8IBYYceZhJ+Tf8gU9sr6LZqen524pNyeAR0BX7LKymw6MBzE/OHD/rWwb8CQA1dPOUCGOrKYyiLpg64c/JsvzP3m8bU+TgoyM6INtSWKMUG64u1VHNVyEhG6678eNuEnk9vo1DaPxm+hQ/+6vjfB2goVzlDc7XxAwMis9WWgontBXMmltVo8EUdFXQALsLE74dnZYz2p5d4POgi0TSdgkCpSKEPmKz37zuTQuqQedl7";

/// - asset: BTC-USD
/// - id: `0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43`
/// - price: `7140518251230`
/// - publish_time: `1730209108` (older than `VAA_1`)
const VAA_2: &str = "UE5BVQEAAAADuAEAAAAEDQCjLDRdqK3N7OQYMpyXtLW6NKSo3YCfYuACOY0+dqOVhFSUy/G9IE2V4lAtgw45G+gnTiYzvqAzI+Nsn/CqiXhTAAIG9CeuTb3mHRhTMvuKIovaNiaqGByiEcGgktVrzGFDuiDUCz6gLHYpo4fvMu3VntsrR0D/fJEE6MKsQ5fkqnxmAAP8omFTZqYdLehi92469mNVP34lPIvgKe+4uS+DoGSEHCaQ3FvdCwNh/wrzlYzSIfCSpR4wiRmDQD8M3OGjTq8SAAS9mHRePZ9DIkuKNKdTNJg4Sfj4v8P+tgI8P12sEkrMkAoBeRe3SDs3+E+bp2QF9vD2zbkpZ8CFLeGykR3rRCqyAAbSPt1zSDnqW60YvS6irWN37lN9Z7PDs5n0pk/52Zq5lS2eni8JZrw3CMu/T0sHC1eLwzXOSztGlRj+6HwvEFOqAAiJzRd5pn0sZbv2vvv6Ot2+E/ZYSaSSz1MO2p3BGDpWomVqDPCyMRsSZHOqAs1YCPHD1fSspK1IBw+EWdSWOrILAAqe3YuGv5rwe2Kk8FWKpgAyI0+0R9CYm3Kt6GOpixST2zLdOFymTCYmVUbmPUVjlB+6Fb0jo4IVXkOT19mZkMqKAAvyP9U1p4QaaU52R1sod0Fj8Ub2MWJGsxpJ841cH2h6FmteTBYLft8NKW2GIatJsEOtylVoT6fvHTgz8A4wgf1VAQw4HfkNRPXNRdyQLNlkTFbAk/kSHLL1UD8nKCauZHoGonNAWh9qvZimaGfmOD4uiKppUd+ZhMMvsXDtijNzhuy6AQ2clWsOjtX/cIHYXKQarWNkjpAcTGtYxWKvUR3HKm1xLyix6MTMBDe8DgsUu64bK+9h6NzJERgrKTn6LPCmdelZAQ6SU9niLq7hdRssDBOOWq1MoCTVlrIRgj1PzU2cz1IqsUrX6FNZKzy7TR1aEGAGdP4F5HZQ8EDG3Asd1g7cfX/OAQ8O+izE/1/ehjrpaUkbswwEPWZUjlRaWlDA/tfxHGuhEXEoQA9XNObJbVlTnQ5qIMG8L9IpHeiscg/ix7qlVBTCARHn1hu4yxqwmE2KnxckZ27glAFIzWnGw4lmM0LJ2bwF7XLprLSpbtWepW++FXiXQZyIN9w+WWmq/SmbN8qWvBgkAGcg5VQAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFWSo0AUFVV1YAAAAAAApl86oAACcQyn8+aUYc39L5VyEjcX80wtkEAzEBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAZ+iA3K3gAAAAEkZi9F////+AAAAABnIOVUAAAAAGcg5VMAAAZ5MEd5gAAAAAEC5k7gCtpb6OPLmKjcp4u0Hp6m6S4AMylc/ct/p8EMdXifhgspJtnrp8/EAkaySBRkp3r0WWDhLdjMneaKbjE2JI1pP52yG4Fpuq5NDMRdZDUXZZE00xaK2MFLQ0YgHU7ik6wSd0WMYkcni/OpimZSpiWoGl9LZnGwfpXjRyLeKGU6BCG38rquwNi32y2fAzvgf5FUUVDVC3WQDnWUB4+J/nbP646X24XJwE+ecDE/AMNwYki4Eq+Rhnobfk7JItlRJiqiCLcJC7CMeHiY";

/// BTC_USD_ID
/// This data have same `publish_time` but different price and different sequence,
/// to test the update logic inside the oracle.
const OLD_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQC1PrJvEJdrAATLXCaO9KPKy8jg60pFB+/dG5WeLIjHyw8uUxNF/UQ54R2oBxGF97NHwHRZtF0/Sra0XBijMAuOAALeIiPvZN3bcnWRMAqSwZCUYN/SZ79xHQhJQ07rM85SHVzUYMo5LCiuV1h0O9tvA5kmyABfyV2a3Y7eQZoroInKAAN6LJGqKqSo6ZaSCMVBZ+6tQ+Tr4+IgUrzlxIg/kCfZoVZbgTKiBmt1iDhcLkwC+bWOxHpHloqISFpPJOXDuCBGAAT9vCGgUCSflqT/muUoQ5qFtC14RaTWjVw6CKINncZntA2cN/yJw6A+Xs5U/DU+bmN35PxGzD6q7FjFb5X9qbqhAQZ/wwZDiZCdVa3ra9I6CQ+e6SSulXxUTj/bZFd+5ijKpR0CCOIa+aqYIUyACSGjh2HZdo4b74kGEWmBU84qHkszAAg1p30sJ8vyw9W38JrZd+OnOBfxiii0Y12+xZWbzdWrfibFvYiuJkxGdWm+G5pUie+hrgC8rv/Gr37Vup6U2JOIAQvMUewdWzGDQ4RdJlG9JE46eZwY6+tXVvcPN3HWGpialyJUtpbFzU2A+lYiP1U+me1HX+LsUXLLgtxCyUYfSvY1AQwBEE2estPGKI3D55X+U5lazI5vLEbqN+3Ek9HfChwr0VDHpvtOcxvxFAuC7A05KkgtJ2DlDGqQzjSCMIqLunhLAQ0ImWBmzfbh71buxjRd3pcx2u0vBr22b3hZmdvgosd4kQUYiXmQqqSdYEUAsYsI8lS6IY5tKmmt5Ne3Db1cfkLiAQ5fwFgtmfxP9vhNzQH9wSDy3uGWeTY8QT6Nth4WorwqPwHIoLJ5GoPQmhmQ+Lamm1iRZkHkeiWMcJFmrXvjMs60AQ9t6nrW9X8iPHXhJidZkc30tXZTbte6HFu4wd+d2r9P/DrmqeuX0Z1ImR1h3PXhO8+llZcYymepesPG7oYYdNebARDK6B2HQ+YqZy39uLSvv9Ixc5PAFVE3Es69ZOIIzyNqPhO3XnlPYBPpBuC9bzXhxacenfk9++YnO7wsVTAjarfmARG5FUbdJ3/pnJC1hfeXgDg9ZWu1vhXSJAPgOpJLlIhvaTgZk7JWSMBaOALtD8yaea6t16LM4XDzdeWrUdGFgBLAAGfUI2QAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAHG0BzAUFVV1YAAAAAAAwoR/4AACcQHJX9oR7eAJ3kDAJmGcmHXSG1M8QBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAeXyVYyXAAAAAC5aSAk////+AAAAABn1CNkAAAAAGfUI2MAAAeRixECAAAAAAC1MsVIDGVIb4bQKzzTbGYwasa37r9mN/OGqfaDpf2gg0Je0EyRbd7dDLTzEhSNqJBsntjW9sMHHu0TOkqpJI7ygjPSNAO2qbAU4lj8KAUFRZ1hP4SISP2UX0V5NrhALGOCWnTQErcCdy6Gm6orqVJhq/jLQ0Jljvzq9N38b/tmDKeyzJl3MufHg80fBOppdw5W4zJg54UZl1gqb6JGeE5vMPDoze5fZ2UbOWtcwS/HfHEq0JvH3z9zbS5Kt8brXfQbODqiWLmHQPIf+w+xe147+bsQ/2jd3Kigq7h6O1wbrI8xHT1QhxhdBItjPhUO7U6SCAW+5A==";
const NEW_VAA: &str = "UE5BVQEAAAADuAEAAAAEDQDJVzkTXJ/xX0Z6yCF9CeCDwGIskGvr0dgHi5DHq+ZiWHS/fqNgstRz7U0k50dTMJak7JOmQePRMWa0abZ4OQv8AQJEfiU38lpjC5mwmbFkdiNU7M3FxdcmEyXqCWHclPbJEDyfgDYamXDQfIbsEzjtf959ZGhbqjmqWQ86zkOcU6pEAANeXPg2UPUQvAiP1l6HDQhkcgu8VHiNJorAWboTw+LwtwotgE4JuvZFCCcNI/gbdFND+Cf7kDulUMd+SNXxlcWLAASXfzh0K3S79739gQXlJ7lK5jdv33I4U70Ma+5COdN5MkhPP+wcYc4ZNCNkC6GhwMknxmbDGcxUVlge9CmdmBfLAAYQ/2bhKpo4fdPHExfK9gl74JGIDkkJMv8D9OXjVQBVEnutA2ucSYtFFmdPoFW8Od4k5vrO0XiVnpajYdhtA/DNAQh0nCn4dmbGvp5uQfRTQ+b7IivCtOSusaIIlN0ippA1E0pxERsyngqICHH4NJazflsgqWbo6KxzuTf7UkMEEF9gAQrhy+VQDRgIjUns3BLueS3DstsrBkKaglXFtqjM+GSU5l+8vqGOSl2gLGU3KCOLYt5FKzrr9BenG+Fr2DKfHHuQAAwPN3YKI48TY7o0d7dKws5Uv9IG2ILMF7+SZU1gd4/ydjSpYADn6msqab1d9+q+tkaW5DPi1f+p93Wh3Kr8eYygAQ3LCDbzf7z/nsDesRUmEyv944SYqa+AHOEWDAMhNfu6qgM8DkvJROK6vcgF9+SrfJN/0W+l3RCR1GfRyTMSMFLtAA6n1JaDtwLbcmgmCfnDIzeP2HF8LktHJiHNxSbAtFLFjneL+8fz69eSBqEX8Szm7vWWTkuVzLfVPoqfSzDsFdsNAA8ds4VTQTUvVMTRr6OUDkYZmFFHf9S6Z9Aan94Fr08CT0k7jv80Csk6sfa55Zd5e+/llRR4YOVtC0SXfKtmhmnIABDIgEd/lNqC8c8s5A9sJwklvjbdDheNroZ1A3hCBq017CqGHjweC1Cd3byISTVpV+Rpl4Hg8VK3R+PgaNwRwT1vARHeO2usqEiKywPA+GgMGfQSYOkYXQkzZ80CB0i37KHgIlbqrDaBYzE/WD/0ugcetCE2saF2quy2+eVddIeazgfWAGfUI2QAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAHG0B0AUFVV1YAAAAAAAwoR/8AACcQ6c3sPfMBZYx5zE2zbC46nih06gIBAFUA5i32yLSoX+GmfbRNwS3l2zMPesZrctxliv7fD0pBW0MAAAeXzlFGBwAAAAC7lRp5////+AAAAABn1CNkAAAAAGfUI2QAAAeRi0Zp4AAAAAC1MvW4DEFMNDUv+h3foglPFliT76Pd9cXmhIjNtClQyN06tX+QlUXpaO/a8IRr/9eZMk1j8T/WdU1ITKEvwDSbkpCWAsqvPeRstBPAtUZvVYk9BkTVCcES9uDSDc9jFPlJBp23zY1+TZb1wC8bZjn+qhDAM7kytlpRBMLGLuJWmqGEOheF82gcxQNlG+V+sh9lxwHJekKJiW0Ni/PxM53JqM/o6M+MtkYZIgLRL8nrDSGG4DxSJ/iIvl9fgTfa8tmWGI+pw96LtQw4GywuNjFAKbdHpaHNSiV/uNB22f7MoV1I8rscwQLmrRbopdKLYQRHGk3apg==";

fn setup_oracle_test() -> (TestSuite<NaiveProposalPreparer>, TestAccounts, Addr) {
    let (suite, accounts, _, contracts, _) = setup_test_naive(Default::default());
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
                denom: btc::DENOM.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec256::from_str("71405.18251230").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec256::from_str("71175.70800000").unwrap()
        );

        assert_eq!(current_price.precision(), 8);

        assert_eq!(current_price.timestamp, Timestamp::from_seconds(1730209108));
    }

    // Push an updated price
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
                denom: btc::DENOM.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec256::from_str("68744.84759622").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec256::from_str("68739.90600000").unwrap()
        );

        assert_eq!(current_price.timestamp, Timestamp::from_seconds(1730804420));
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
                denom: btc::DENOM.clone(),
            })
            .unwrap();

        assert_eq!(
            current_price.humanized_price,
            Udec256::from_str("68744.84759622").unwrap()
        );

        assert_eq!(
            current_price.humanized_ema,
            Udec256::from_str("68739.90600000").unwrap()
        );

        assert_eq!(current_price.timestamp, Timestamp::from_seconds(1730804420));
    }
}

#[test]
fn multiple_vaas() {
    let (mut suite, mut accounts, oracle) = setup_oracle_test();

    let pyth_client = PythClientCache::new(PYTH_URL).unwrap();

    let id_denoms = btree_map! {
        ATOM_USD_ID => atom::DENOM.clone(),
        BNB_USD_ID  => bnb::DENOM.clone(),
        BTC_USD_ID  => btc::DENOM.clone(),
        DOGE_USD_ID => doge::DENOM.clone(),
        ETH_USD_ID  => eth::DENOM.clone(),
        SOL_USD_ID  => sol::DENOM.clone(),
        USDC_USD_ID => usdc::DENOM.clone(),
        XRP_USD_ID  => xrp::DENOM.clone(),
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
fn sequence() {
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
