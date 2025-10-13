use {
    dango_client::SingleSigner,
    dango_types::constants::btc,
    grug::{
        Addr, BroadcastClientExt, GasOption, JsonSerExt, QueryClientExt, SearchTxClient,
        TendermintRpcClient, addr, coins,
    },
    hex_literal::hex,
    indexer_client::HttpClient,
    std::time::Duration,
};

const CHAIN_ID: &str = "dev-6";

const OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const BOT: Addr = addr!("bed1fa8569d5a66935dea5a179b77ac06067de32");

const BOT_USERNAME: &str = "user8";

/// For demonstration purpose only; do not use this in production.
const BOT_PRIVATE_KEY: [u8; 32] =
    hex!("ca956fcf6b0f32975f067e2deaf3bc1c8632be02ed628985105fd1afc94531b9");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("http://api.testnet.ovh1.dango.zone")?;
    // let client = TendermintRpcClient::new("http://ovh1:36657")?;

    let mut bot = SingleSigner::from_private_key(BOT_USERNAME, BOT, BOT_PRIVATE_KEY)?
        .with_query_nonce(&client)
        .await?;

    let balances_before = client.query_balances(BOT, None, None, None).await?;
    println!("bot balances before: {balances_before}");

    let outcome = client
        .transfer(
            &mut bot,
            OWNER,
            coins! { btc::DENOM.clone() => 123 },
            GasOption::Predefined { gas_limit: 100_000 },
            CHAIN_ID,
        )
        .await?;
    // println!("{}", outcome.to_json_string_pretty()?);

    tokio::time::sleep(Duration::from_secs(2)).await;

    let tx = client.search_tx(outcome.tx_hash).await?;
    println!("{}", tx.to_json_string_pretty()?);

    let balances_after = client.query_balances(BOT, None, None, None).await?;
    println!("bot balances after: {balances_after}");

    Ok(())
}
