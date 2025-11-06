use {
    dango_client::SingleSigner,
    grug::{Addr, BroadcastClientExt, GasOption, JsonSerExt, QueryClientExt, SearchTxClient, addr},
    grug_app::GAS_COSTS,
    hex_literal::hex,
    indexer_client::HttpClient,
    std::time::Duration,
};

const CHAIN_ID: &str = "dev-9"; // devnet = dev-9, testnet = dev-6

const OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const OWNER_USERNAME: &str = "owner";

/// For demonstration purpose only; do not use this in production.
const OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://api-devnet.dango.zone/")?;

    let mut owner = SingleSigner::from_private_key(OWNER_USERNAME, OWNER, OWNER_PRIVATE_KEY)?
        .with_query_nonce(&client)
        .await?;

    let outcome = client
        .upgrade(
            &mut owner,
            "0.1.0", // should match the cargo version of dango-cli
            2551000,
            None::<String>,
            None::<String>,
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            CHAIN_ID,
        )
        .await?;
    println!("tx broadcasted:\n{}", outcome.tx_hash);

    println!("\nwaiting 5 seconds for transaction to confirm...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let outcome = client.search_tx(outcome.tx_hash).await?;
    println!("\ntransaction:\n{}", outcome.to_json_string_pretty()?);

    let next_upgrade = client.query_next_upgrade(None).await?;
    println!("\nnext upgrade:\n{}", next_upgrade.to_json_string_pretty()?);

    Ok(())
}
