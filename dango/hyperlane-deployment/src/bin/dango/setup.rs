use {
    crate::config::Config,
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_types::auth::Nonce,
    grug::{Addr, Defined, addr},
    hex_literal::hex,
    indexer_client::HttpClient,
};

use crate::config;

const DANGO_OWNER_USERNAME: &str = "owner";
const DANGO_OWNER_ADDR: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");
const DANGO_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

pub async fn setup() -> anyhow::Result<(HttpClient, SingleSigner<Secp256k1, Defined<Nonce>>, Config)>
{
    let config = config::load_config()?;

    let dango_client = HttpClient::new(&config.dango_api_url)?;

    let dango_owner = SingleSigner::new(
        &DANGO_OWNER_USERNAME,
        DANGO_OWNER_ADDR,
        Secp256k1::from_bytes(DANGO_OWNER_PRIVATE_KEY)?,
    )?
    .with_query_nonce(&dango_client)
    .await?;

    Ok((dango_client, dango_owner, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup() {
        setup().await.unwrap();
    }
}
