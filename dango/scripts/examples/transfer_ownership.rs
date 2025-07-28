use {
    dango_client::SingleSigner,
    dango_types::config::AppConfig,
    grug::{Addr, BroadcastClientExt, ClientWrapper, GasOption, JsonSerExt, QueryClientExt, addr},
    grug_app::GAS_COSTS,
    hex_literal::hex,
    indexer_client::HttpClient,
    std::sync::Arc,
};

const CHAIN_ID: &str = "dev-6";

const CURRENT_OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

const CURRENT_OWNER_USERNAME: &str = "owner";

/// For demonstration purpose only; do not use this in production.
const CURRENT_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const NEW_OWNER: Addr = addr!("747a4f43c538ac55445bc948209cf2c05855f584");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ClientWrapper::new(Arc::new(HttpClient::new("http://ovh2:8080")));

    let mut current_owner = SingleSigner::from_private_key(
        CURRENT_OWNER_USERNAME,
        CURRENT_OWNER,
        CURRENT_OWNER_PRIVATE_KEY,
    )?
    .query_nonce(&client)
    .await?;

    let mut cfg = client.query_config(None).await?;
    cfg.owner = NEW_OWNER;

    let outcome = client
        .configure(
            &mut current_owner,
            Some(cfg),
            None::<AppConfig>,
            GasOption::Simulate {
                scale: 2.,
                flat_increase: GAS_COSTS.secp256k1_verify,
            },
            CHAIN_ID,
        )
        .await?;
    println!("{}", outcome.to_json_string_pretty()?);

    Ok(())
}
