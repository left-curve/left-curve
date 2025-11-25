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
        primitives::{Address, FixedBytes, U256},
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
        signers::local::{MnemonicBuilder, coins_bip39::English},
        sol_types::SolCall,
    },
    dango_scripts::{
        addresses::sepolia::{HYPERLANE_MAILBOX, erc20s, hyperlane_deployments::eth},
        contract_bindings::{
            hyp_erc20_collateral::HypERC20Collateral,
            hyp_native::HypNative,
            proxy::{ProxyAdmin, TransparentUpgradeableProxy},
        },
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Inner, QueryClientExt},
    indexer_client::HttpClient,
    std::env,
};

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";

/// The ERC20 address which will be wrapped by the HWR. If set to None the HWR will be deployed as a HypNative contract.
/// If provided, the HWR will be deployed as a HypERC20Collateral contract.
const ERC20_ADDRESS: Option<Address> = Some(erc20s::USDC);

/// The proxy admin address to use. If set to None, a new ProxyAdmin contract will be deployed.
const PROXY_ADMIN_ADDRESS: Option<Address> = Some(eth::PROXY_ADMIN);

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

    // Deploy new ProxyAdmin contract if not provided.
    let admin = if let Some(proxy_admin_address) = PROXY_ADMIN_ADDRESS {
        println!("Using existing ProxyAdmin contract: {proxy_admin_address}");
        ProxyAdmin::new(proxy_admin_address, &provider)
    } else {
        println!("Deploying new ProxyAdmin contract...");
        let admin = ProxyAdmin::deploy(&provider).await?;
        println!("Done! ProxyAdmin address: {}", admin.address());
        admin
    };

    let proxy = if let Some(erc20_address) = ERC20_ADDRESS {
        println!("Deploying HypERC20Collateral contract for {erc20_address}");
        let hyperlane_warp_route =
            HypERC20Collateral::deploy(&provider, erc20_address, U256::ONE, HYPERLANE_MAILBOX)
                .await?;
        println!("Done! HWR address: {}", hyperlane_warp_route.address());

        println!("Deploying proxy contract...");
        let proxy = TransparentUpgradeableProxy::deploy(
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
        println!("Done! Proxy address: {}", proxy.address());

        proxy
    } else {
        println!("Deploying HypNative contract...");
        let hyperlane_warp_route =
            HypNative::deploy(&provider, U256::ONE, HYPERLANE_MAILBOX).await?;
        println!("Done! HWR address: {}", hyperlane_warp_route.address());

        println!("Deploying proxy contract...");
        let proxy = TransparentUpgradeableProxy::deploy(
            &provider,
            *hyperlane_warp_route.address(),
            *admin.address(),
            HypERC20Collateral::initializeCall {
                _hook: Address::ZERO,                     // use mailbox default
                _interchainSecurityModule: Address::ZERO, // use mailbox default
                _owner: owner,
            }
            .abi_encode()
            .into(),
        )
        .await?;
        println!("Done! Proxy address: {}", proxy.address());

        proxy
    };

    println!("Enrolling dango domain in router...");
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*proxy.address())
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
