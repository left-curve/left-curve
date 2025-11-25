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
        primitives::{FixedBytes, U256},
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
        signers::local::{MnemonicBuilder, coins_bip39::English},
    },
    dango_scripts::{
        addresses::sepolia::hyperlane_deployments::eth,
        contract_bindings::{
            hyp_erc20_collateral::HypERC20Collateral, proxy::TransparentUpgradeableProxy,
        },
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Addr, Inner, QueryClientExt, addr},
    indexer_client::HttpClient,
    std::env,
};

/// The required hook on Sepolia is set to the `ProtocolFee` contract which
/// charges a static fee of 1 wei for all outgoing transfers.
const SEPOLIA_PROTOCOL_FEE: U256 = U256::from_le_slice(&[1]);

// The coin to warp. Ether "eth" or USDC "usdc".
const WARP_AMOUNT: u64 = 100;

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";
const DANGO_RECIPIENT: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

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

    // Setup ethereum provider
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

    // Setup contracts
    let warp_route_proxy = TransparentUpgradeableProxy::new(eth::WARP_ROUTE_PROXY, &provider);

    println!("warping {} wei to dango...", WARP_AMOUNT);
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*warp_route_proxy.address())
                .with_value(U256::from(WARP_AMOUNT) + SEPOLIA_PROTOCOL_FEE)
                .with_call(&HypERC20Collateral::transferRemoteCall {
                    _destination: dango_domain,
                    _recipient: FixedBytes::<32>::left_padding_from(DANGO_RECIPIENT.inner()),
                    _amount: U256::from(WARP_AMOUNT),
                }),
        )
        .await?
        .watch()
        .await?;
    println!("done! tx hash: {tx_hash}");

    Ok(())
}
