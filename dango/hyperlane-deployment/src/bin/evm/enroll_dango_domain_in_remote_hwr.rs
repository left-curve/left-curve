//! This script deploys the `HypNativeMetadata` contract as an upgradeable proxy
//! on Sepolia, configure the router, and sends 100 wei to a recipient on Dango.
//!
//! Prerequisite: create a `.env` file at the repository root, with the
//! following content:
//!
//! ```plain
//! INFURA_API_KEY="your_infura_api_key"
//! MNEMONIC="your_mnemonic"
//! ```

use {
    alloy::{
        network::{EthereumWallet, TransactionBuilder},
        primitives::{Address, FixedBytes},
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
        signers::local::{MnemonicBuilder, coins_bip39::English},
    },
    dango_hyperlane_deployment::{
        addresses::sepolia::hyperlane_deployments::eth,
        contract_bindings::{hyp_native::HypNative, proxy::TransparentUpgradeableProxy},
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Inner, QueryClientExt},
    indexer_client::HttpClient,
    std::env,
};

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";

/// The proxy contract on Sepolia for which to enroll the Dango domain.
const PROXY: Address = eth::WARP_ROUTE_PROXY;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    // Setup Ethereum provider
    let infura_api_key = env::var("INFURA_API_KEY")?;
    let url = format!("https://sepolia.infura.io/v3/{infura_api_key}");

    let mnemonic = env::var("SEPOLIA_MNEMONIC").unwrap();
    let signer = MnemonicBuilder::<English>::default()
        .phrase(&mnemonic)
        .build()
        .unwrap();

    let owner = signer.address();
    println!("using {owner} as owner address");

    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::new(signer))
        .connect_http(url.parse().unwrap());

    // Setup Dango client
    let dango_client = HttpClient::new(DANGO_API_URL)?;
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // Query the warp contract address from the app config and pad it to 32 bytes.
    let warp_contract_address = app_cfg.addresses.warp;
    let warp_contract_address_fixed =
        FixedBytes::<32>::left_padding_from(warp_contract_address.inner());

    // Query the mailbox config to get the dango domain
    let mailbox_config = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.mailbox,
            hyperlane_types::mailbox::QueryConfigRequest {},
            None,
        )
        .await?;
    let dango_domain = mailbox_config.local_domain;

    // Setup contract wrapper
    let hwr_proxy = TransparentUpgradeableProxy::new(PROXY, &provider);

    println!("Enrolling dango domain in router on Sepolia...");
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*hwr_proxy.address())
                .with_call(&HypNative::enrollRemoteRouterCall {
                    _domain: dango_domain,
                    _router: warp_contract_address_fixed,
                }),
        )
        .await?
        .watch()
        .await?;
    println!("Done! tx hash: {tx_hash}");

    Ok(())
}
