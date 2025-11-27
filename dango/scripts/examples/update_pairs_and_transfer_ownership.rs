use {
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_types::{
        constants::{btc, btc_usd, dango, eth, eth_usd, sol, sol_usd, usd},
        dex::{self, Geometric, PairParams, PairUpdate, PassiveLiquidity, Xyk},
    },
    grug::{
        Addr, Bounded, BroadcastClientExt, Coins, Denom, GasOption, JsonSerExt, Message, NonEmpty,
        QueryClientExt, SearchTxClient, Udec128, Uint128, addr, btree_set,
    },
    grug_app::GAS_COSTS,
    hex_literal::hex,
    indexer_client::HttpClient,
    std::{collections::BTreeSet, str::FromStr, time::Duration},
};

const CHAIN_ID: &str = "dev-6";

const CURRENT_OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const CURRENT_OWNER_USERNAME: &str = "owner";

/// For demonstration purpose only; do not use this in production.
const CURRENT_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

const NEW_OWNER: Addr = addr!("df8dbf9a60758b9913665b115174e0427202046b");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://api-testnet.dango.zone/")?;

    let mut owner = SingleSigner::new(
        CURRENT_OWNER_USERNAME,
        CURRENT_OWNER,
        Secp256k1::from_bytes(CURRENT_OWNER_PRIVATE_KEY)?,
    )?
    .with_query_nonce(&client)
    .await?;

    let current_cfg = client.query_config(None).await?;

    let outcome = client
        .send_messages(
            &mut owner,
            NonEmpty::new_unchecked(vec![
                Message::execute(
                    DEX,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![
                        PairUpdate {
                            base_denom: dango::DENOM.clone(),
                            quote_denom: usd::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/dango/usd")?,
                                pool_type: PassiveLiquidity::Xyk(Xyk {
                                    spacing: Udec128::new_bps(10),
                                    reserve_ratio: Bounded::new_unchecked(Udec128::new_percent(1)),
                                    limit: 30,
                                }),
                                bucket_sizes: BTreeSet::new(),
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                        PairUpdate {
                            base_denom: btc::DENOM.clone(),
                            quote_denom: usd::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/btc/usd")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::new_bps(10), // means 10 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                }),
                                bucket_sizes: btree_set! {
                                    btc_usd::ONE_HUNDREDTH,
                                    btc_usd::ONE_TENTH,
                                    btc_usd::ONE,
                                    btc_usd::TEN,
                                    btc_usd::FIFTY,
                                    btc_usd::ONE_HUNDRED,
                                },
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                        PairUpdate {
                            base_denom: eth::DENOM.clone(),
                            quote_denom: usd::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/eth/usd")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::from_str("0.000000000005")?, // means 5 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                }),
                                bucket_sizes: btree_set! {
                                    eth_usd::ONE_HUNDREDTH,
                                    eth_usd::ONE_TENTH,
                                    eth_usd::ONE,
                                    eth_usd::TEN,
                                    eth_usd::FIFTY,
                                    eth_usd::ONE_HUNDRED,
                                },
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                        PairUpdate {
                            base_denom: sol::DENOM.clone(),
                            quote_denom: usd::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/sol/usd")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::new_bps(10), // means 1 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                }),
                                bucket_sizes: btree_set! {
                                    sol_usd::ONE_HUNDREDTH,
                                    sol_usd::ONE_TENTH,
                                    sol_usd::ONE,
                                    sol_usd::TEN,
                                },
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                    ])),
                    Coins::new(),
                )?,
                Message::configure(
                    Some(grug::Config {
                        owner: NEW_OWNER,
                        ..current_cfg
                    }),
                    None::<dango_types::config::AppConfig>,
                )?,
            ]),
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            CHAIN_ID,
        )
        .await?;
    println!("tx broadcasted: {}", outcome.tx_hash);

    tokio::time::sleep(Duration::from_secs(1)).await;

    let outcome = client.search_tx(outcome.tx_hash).await?;
    println!("{}", outcome.to_json_string_pretty()?);

    Ok(())
}
