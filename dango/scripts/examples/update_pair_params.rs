use {
    dango_client::SingleSigner,
    dango_types::dex::{self, Geometric, PassiveLiquidity},
    grug::{
        Addr, Bounded, BroadcastClientExt, Coins, GasOption, JsonSerExt, QueryClientExt, Udec128,
        addr,
    },
    hex_literal::hex,
    indexer_client::HttpClient,
    std::str::FromStr,
};

const CHAIN_ID: &str = "dev-6";

const OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const OWNER_USERNAME: &str = "owner";

/// For demonstration purpose only; do not use this in production.
const OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://testnet.dango.exchange")?;

    let mut owner = SingleSigner::from_private_key(OWNER_USERNAME, OWNER, OWNER_PRIVATE_KEY)?
        .with_query_nonce(&client)
        .await?;

    // Query the current pair params.
    let mut pairs = client
        .query_wasm_smart(
            DEX,
            dex::QueryPairsRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    // Update each pool type to geometric with sensible parameters.
    for pair in &mut pairs {
        pair.params.pool_type = PassiveLiquidity::Geometric(Geometric {
            spacing: Udec128::from_str("0.0001")?,
            ratio: Bounded::new_unchecked(Udec128::from_str("0.8")?),
            limit: 3,
        });
        pair.params.swap_fee_rate = Bounded::new_unchecked(Udec128::from_str("0.0001")?);
    }

    let outcome = client
        .execute(
            &mut owner,
            DEX,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(pairs)),
            Coins::new(),
            GasOption::Predefined { gas_limit: 100_000 },
            CHAIN_ID,
        )
        .await?;
    println!("{}", outcome.to_json_string_pretty()?);

    Ok(())
}
