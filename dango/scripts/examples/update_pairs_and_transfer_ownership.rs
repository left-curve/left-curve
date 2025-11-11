use {
    dango_client::SingleSigner,
    dango_types::{
        constants::{btc, btc_usdc, dango, eth, eth_usdc, sol, sol_usdc, usdc},
        dex::{
            self, AvellanedaStoikovParams, Geometric, PairParams, PairUpdate, PassiveLiquidity, Xyk,
        },
    },
    grug::{
        Addr, Bounded, BroadcastClientExt, Coins, Dec, Denom, GasOption, JsonSerExt, Message,
        NonEmpty, NumberConst, QueryClientExt, SearchTxClient, Udec128, Uint128, addr, btree_set,
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

    let mut owner = SingleSigner::from_private_key(
        CURRENT_OWNER_USERNAME,
        CURRENT_OWNER,
        CURRENT_OWNER_PRIVATE_KEY,
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
                            quote_denom: usdc::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/dango/usdc")?,
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
                            quote_denom: usdc::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/btc/usdc")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::new_bps(10), // means 10 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                    avellaneda_stoikov_params: AvellanedaStoikovParams {
                                        // gamma ≈ swap_fee_rate * price_in_base_units
                                        // For BTC/USDC: price_in_base_units ≈ $100k * 10^6 / 10^8 = 1.0
                                        // gamma ≈ 0.003 * 1.0 = 0.003
                                        gamma: Dec::from_str("0.003").unwrap(),
                                        time_horizon: grug::Duration::from_seconds(0),
                                        k: Dec::ONE,
                                        half_life: grug::Duration::from_seconds(30),
                                        base_inventory_target_percentage: Bounded::new(
                                            Udec128::new_percent(50),
                                        )
                                        .unwrap(),
                                    },
                                }),
                                bucket_sizes: btree_set! {
                                    btc_usdc::ONE_HUNDREDTH,
                                    btc_usdc::ONE_TENTH,
                                    btc_usdc::ONE,
                                    btc_usdc::TEN,
                                    btc_usdc::FIFTY,
                                    btc_usdc::ONE_HUNDRED,
                                },
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                        PairUpdate {
                            base_denom: eth::DENOM.clone(),
                            quote_denom: usdc::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/eth/usdc")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::from_str("0.000000000005")?, // means 5 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                    avellaneda_stoikov_params: AvellanedaStoikovParams {
                                        // gamma ≈ swap_fee_rate * price_in_base_units
                                        // For ETH/USDC: price_in_base_units ≈ $4k * 10^6 / 10^18 = 0.000000004
                                        // gamma ≈ 0.003 * 0.000000004 = 0.000000000012
                                        gamma: Dec::from_str("0.000000000012").unwrap(),
                                        time_horizon: grug::Duration::from_seconds(0),
                                        k: Dec::ONE,
                                        half_life: grug::Duration::from_seconds(30),
                                        base_inventory_target_percentage: Bounded::new(
                                            Udec128::new_percent(50),
                                        )
                                        .unwrap(),
                                    },
                                }),
                                bucket_sizes: btree_set! {
                                    eth_usdc::ONE_HUNDREDTH,
                                    eth_usdc::ONE_TENTH,
                                    eth_usdc::ONE,
                                    eth_usdc::TEN,
                                    eth_usdc::FIFTY,
                                    eth_usdc::ONE_HUNDRED,
                                },
                                swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                                min_order_size_base: Uint128::new(5),
                                min_order_size_quote: Uint128::new(5),
                            },
                        },
                        PairUpdate {
                            base_denom: sol::DENOM.clone(),
                            quote_denom: usdc::DENOM.clone(),
                            params: PairParams {
                                lp_denom: Denom::from_str("dex/pool/sol/usdc")?,
                                pool_type: PassiveLiquidity::Geometric(Geometric {
                                    spacing: Udec128::new_bps(10), // means 1 USDC
                                    ratio: Bounded::new_unchecked(Udec128::new_percent(60)),
                                    limit: 5,
                                    avellaneda_stoikov_params: AvellanedaStoikovParams {
                                        // gamma ≈ swap_fee_rate * price_in_base_units
                                        // For SOL/USDC: price_in_base_units ≈ $200 * 10^6 / 10^9 = 0.0002
                                        // gamma ≈ 0.003 * 0.0002 = 0.0000006
                                        gamma: Dec::from_str("0.0000006").unwrap(),
                                        time_horizon: grug::Duration::from_seconds(0),
                                        k: Dec::ONE,
                                        half_life: grug::Duration::from_seconds(30),
                                        base_inventory_target_percentage: Bounded::new(
                                            Udec128::new_percent(50),
                                        )
                                        .unwrap(),
                                    },
                                }),
                                bucket_sizes: btree_set! {
                                    sol_usdc::ONE_HUNDREDTH,
                                    sol_usdc::ONE_TENTH,
                                    sol_usdc::ONE,
                                    sol_usdc::TEN,
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
