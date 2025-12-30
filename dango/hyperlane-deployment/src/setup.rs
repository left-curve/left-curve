use {
    crate::config::dango::DangoConfig,
    dango_client::{Secp256k1, Secret, SingleSigner},
    grug::{Addr, addr},
    hex_literal::hex,
    indexer_client::HttpClient,
};

const DANGO_OWNER_ADDR: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

// Demo purpose only. Do not use for production!
const DANGO_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const DANGO_USER5_ADDR: Addr = addr!("a20a0e1a71b82d50fc046bc6e3178ad0154fd184");

// Demo purpose only. Do not use for production!
const DANGO_USER5_PRIVATE_KEY: [u8; 32] =
    hex!("fe55076e4b2c9ffea813951406e8142fefc85183ebda6222500572b0a92032a7");

pub async fn setup_dango(
    config: &DangoConfig,
) -> anyhow::Result<(HttpClient, SingleSigner<Secp256k1>)> {
    let dango_client = HttpClient::new(&config.api_url)?;

    let dango_owner = SingleSigner::new(
        DANGO_OWNER_ADDR,
        Secp256k1::from_bytes(DANGO_OWNER_PRIVATE_KEY)?,
    )
    .with_query_user_index(&dango_client)
    .await?
    .with_query_nonce(&dango_client)
    .await?;

    Ok((dango_client, dango_owner))
}

pub async fn get_user5(dango_client: &HttpClient) -> anyhow::Result<SingleSigner<Secp256k1>> {
    SingleSigner::new(
        DANGO_USER5_ADDR,
        Secp256k1::from_bytes(DANGO_USER5_PRIVATE_KEY)?,
    )
    .with_query_user_index(dango_client)
    .await?
    .with_query_nonce(dango_client)
    .await
}

pub mod evm {
    use {
        alloy::{
            network::EthereumWallet,
            primitives::Address,
            providers::{Provider, ProviderBuilder},
            signers::local::{MnemonicBuilder, coins_bip39::English},
        },
        std::env,
    };

    pub fn setup_ethereum_provider(
        infura_rpc_url: &str,
    ) -> anyhow::Result<(impl Provider, Address)> {
        let infura_api_key = env::var("INFURA_API_KEY")?;
        let url = reqwest::Url::parse(infura_rpc_url)?.join(&infura_api_key)?;

        let mnemonic = env::var("EVM_MNEMONIC")?;
        let signer = MnemonicBuilder::<English>::default()
            .phrase(&mnemonic)
            .build()?;

        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::new(signer.clone()))
            .connect_http(url);

        Ok((provider, signer.address()))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    // use crate::config;

    // use super::*;

    // #[tokio::test]
    // async fn test_setup_dango() {
    //     let config = config::load_config().unwrap().dango;
    //     setup_dango(&config).await.unwrap();
    // }
}
