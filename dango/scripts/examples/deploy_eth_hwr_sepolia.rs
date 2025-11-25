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
        primitives::{Address, FixedBytes, U256, address},
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
        signers::local::{MnemonicBuilder, coins_bip39::English},
        sol_types::SolCall,
    },
    dango_scripts::contract_bindings::{
        hyp_native::HypNative,
        proxy::{ProxyAdmin, TransparentUpgradeableProxy},
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Inner, QueryClientExt},
    indexer_client::HttpClient,
    std::env,
};

/// Mailbox contract on Sepolia.
const SEPOLIA_MAILBOX: Address = address!("fFAEF09B3cd11D9b20d1a19bECca54EEC2884766");

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let dango_client = HttpClient::new(DANGO_API_URL)?;
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // Query the mailbox config to get the dango domain
    let mailbox_config = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.mailbox,
            hyperlane_types::mailbox::QueryConfigRequest {},
            None,
        )
        .await?;
    let dango_domain = mailbox_config.local_domain;
    println!("Deploying HWR pointingto Dango domain: {dango_domain}");

    let infura_api_key = env::var("INFURA_API_KEY")?;
    let url = format!("https://sepolia.infura.io/v3/{infura_api_key}");

    let mnemonic = env::var("SEPOLIA_MNEMONIC")?;
    let signer = MnemonicBuilder::<English>::default()
        .phrase(&mnemonic)
        .build()?;

    let owner = signer.address();
    println!("using {owner} as owner address");

    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::new(signer))
        .connect_http(url.parse()?);

    println!("Deploying ProxyAdmin contract...");
    let admin = ProxyAdmin::deploy(&provider).await?;
    println!("Done! ProxyAdmin address: {}", admin.address());

    println!("Deploying HypNativeMetadata logic contract...");
    let hyperlane_warp_route = HypNative::deploy(&provider, U256::ONE, SEPOLIA_MAILBOX).await?;
    println!("Done! HWR address: {}", hyperlane_warp_route.address());

    println!("Deploying HypNativeMetadata proxy contract...");
    let native_proxy = TransparentUpgradeableProxy::deploy(
        &provider,
        *hyperlane_warp_route.address(),
        *admin.address(),
        HypNative::initializeCall {
            _hook: Address::ZERO,                     // use mailbox default
            _interchainSecurityModule: Address::ZERO, // use mailbox default
            _owner: owner,
        }
        .abi_encode()
        .into(),
    )
    .await?;
    println!("Done! Proxy address: {}", native_proxy.address());

    println!("Enrolling dango domain in router...");
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*native_proxy.address())
                .with_call(&HypNative::enrollRemoteRouterCall {
                    _domain: dango_domain,
                    _router: FixedBytes::<32>::left_padding_from(app_cfg.addresses.warp.inner()),
                }),
        )
        .await?
        .watch()
        .await?;
    println!("Done! tx hash: {tx_hash}");

    Ok(())
}
