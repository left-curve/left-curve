use {
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_testing::constants::user4,
    grug::{BroadcastClientExt, Coins, GasOption, JsonSerExt, QueryClientExt, addr},
    grug_app::GAS_COSTS,
    indexer_client::HttpClient,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dango_client = HttpClient::new("https://api-devnet.dango.zone")?;

    let chain_id = dango_client.query_status(None).await?.chain_id;

    let mut dango_signer = SingleSigner::new(
        user4::USERNAME.as_ref(),
        addr!("5a7213b5a8f12e826e88d67c083be371a442689c"),
        Secp256k1::from_bytes(user4::PRIVATE_KEY)?,
    )?
    .with_query_nonce(&dango_client)
    .await?;

    let result = dango_client
        .transfer(
            &mut dango_signer,
            addr!("0fbc6c01f7c334500f465ba456826c890f3c8160"),
            Coins::default(),
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            &chain_id,
        )
        .await?;
    println!("{}", result.to_json_string_pretty()?);

    Ok(())
}
