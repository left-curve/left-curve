//! Scripts are found in the `examples/` directory.

use {
    anyhow::anyhow,
    dango_client::{Keystore, Secp256k1, Secret, SingleSigner},
    grug::{
        Addr, BroadcastClientExt, GasOption, JsonSerExt, Message, QueryClientExt, SearchTxClient,
    },
    grug_app::GAS_COSTS,
    indexer_client::HttpClient,
    std::time::Duration,
};

#[async_trait::async_trait]
pub trait MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message>;
}

pub async fn send_message<T>(api_url: &str, secret_path: &str, sender: Addr) -> anyhow::Result<()>
where
    T: MessageBuilder,
{
    let client = HttpClient::new(api_url)?;

    let status = client.query_status(None).await?;
    println!(
        "Connected to HTTP client. Current height: {}",
        status.last_finalized_block.height
    );

    let secret = {
        let password = dialoguer::Password::new()
            .with_prompt("Enter the password to decrypt the key")
            .interact()?;
        let sk_bytes = Keystore::from_file(secret_path, &password)?;
        Secp256k1::from_bytes(sk_bytes)?
    };

    let mut sender = SingleSigner::new(sender, secret)
        .with_query_user_index(&client)
        .await?
        .with_query_nonce(&client)
        .await?;

    let msg = T::build_message(&client).await?;

    let outcome = client
        .send_message_with_confirmation(
            &mut sender,
            msg,
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
