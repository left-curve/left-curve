use {
    anyhow::anyhow,
    dango_client::{Keystore, Secp256k1, Secret, SingleSigner},
    dango_types::{config::AppConfig, constants::usdc},
    grug::{
        Addr, BroadcastClientExt, GasOption, JsonSerExt, Message, QueryClientExt, SearchTxClient,
        addr, coins,
    },
    grug_app::GAS_COSTS,
    indexer_client::HttpClient,
    std::time::Duration,
};

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new("https://api-mainnet.dango.zone/")?;

    let status = client.query_status(None).await?;
    println!(
        "Connected to HTTP client. Current height: {}",
        status.last_finalized_block.height
    );

    let owner_secret = {
        let password = dialoguer::Password::new()
            .with_prompt("Enter the password to decrypt the key")
            .interact()?;
        let sk_bytes = Keystore::from_file(OWNER_SECRET_PATH, &password)?;
        Secp256k1::from_bytes(sk_bytes)?
    };

    let mut owner = SingleSigner::new(OWNER_ADDRESS, owner_secret)
        .with_query_user_index(&client)
        .await?
        .with_query_nonce(&client)
        .await?;

    let mut app_cfg: AppConfig = client.query_app_config(None).await?;
    app_cfg.minimum_deposit = coins! { usdc::DENOM.clone() => 10000000 };

    let outcome = client
        .send_message_with_confirmation(
            &mut owner,
            Message::configure(None, Some(app_cfg.to_json_value()?))?,
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            &status.chain_id,
            |tx| {
                println!("{}", tx.to_json_string_pretty()?);
                Ok(dialoguer::Confirm::new()
                    .with_prompt("Broadcast transaction?")
                    .interact()?)
            },
        )
        .await?
        .ok_or_else(|| anyhow!("User rejected transaction"))?;
    println!("Tx broadcasted: {}", outcome.tx_hash);

    tokio::time::sleep(Duration::from_secs(1)).await;

    let outcome = client.search_tx(outcome.tx_hash).await?;
    println!("{}", outcome.to_json_string_pretty()?);

    Ok(())
}
