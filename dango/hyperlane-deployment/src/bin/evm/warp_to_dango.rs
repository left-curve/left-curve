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
    },
    dango_hyperlane_deployment::{
        addresses::sepolia::{erc20s::USDC, hyperlane_deployments::usdc},
        contract_bindings::{
            hyp_erc20::HypERC20, hyp_erc20_collateral::HypERC20Collateral,
            proxy::TransparentUpgradeableProxy,
        },
        setup,
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Addr, Inner, QueryClientExt, addr},
    std::env,
};

/// The required hook on Sepolia is set to the `ProtocolFee` contract which
/// charges a static fee of 1 wei for all outgoing transfers.
const SEPOLIA_PROTOCOL_FEE: U256 = U256::from_le_slice(&[1]);

// The coin to warp. Ether "eth" or USDC "usdc".
const WARP_AMOUNT: u64 = 100;

/// The ERC20 address to warp. If set to None, the native coin will be warped.
const WARP_ERC20_ADDRESS: Option<Address> = Some(USDC);

const WARP_ROUTE_PROXY_ADDRESS: Address = usdc::WARP_ROUTE_PROXY;

const DANGO_RECIPIENT: Addr = addr!("a20a0e1a71b82d50fc046bc6e3178ad0154fd184");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let (dango_client, ..) = setup::setup_dango().await?;
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
    let warp_route_proxy = TransparentUpgradeableProxy::new(WARP_ROUTE_PROXY_ADDRESS, &provider);

    let token_name = WARP_ERC20_ADDRESS
        .map(|a| a.to_string())
        .unwrap_or("wei".to_string());
    println!("warping {} {} to dango...", WARP_AMOUNT, token_name);

    let value = if let Some(erc20_address) = WARP_ERC20_ADDRESS {
        println!(
            "approving spend of {} for route proxy ({}) on {}",
            WARP_AMOUNT,
            warp_route_proxy.address().to_string(),
            erc20_address.to_string()
        );
        let tx_hash = provider
            .send_transaction(
                TransactionRequest::default()
                    .with_to(erc20_address)
                    .with_call(&HypERC20::approveCall {
                        spender: *warp_route_proxy.address(),
                        amount: U256::from(WARP_AMOUNT),
                    }),
            )
            .await?
            .watch()
            .await?;
        println!("done! tx hash: {tx_hash}");
        SEPOLIA_PROTOCOL_FEE
    } else {
        U256::from(WARP_AMOUNT) + SEPOLIA_PROTOCOL_FEE
    };

    println!("warping {} {} to dango...", WARP_AMOUNT, token_name);
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*warp_route_proxy.address())
                .with_value(value)
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
