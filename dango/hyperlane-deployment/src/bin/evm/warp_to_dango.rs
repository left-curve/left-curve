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
        network::TransactionBuilder,
        primitives::{Address, FixedBytes, U256},
        providers::Provider,
        rpc::types::TransactionRequest,
    },
    dango_hyperlane_deployment::{
        addresses::sepolia::hyperlane_deployments::usdc,
        config::{self, evm::WarpRouteType},
        contract_bindings::{hyp_erc20::HypERC20, ism::TokenRouter},
        setup,
    },
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{Addr, Inner, QueryClientExt, addr},
};

// The coin to warp. Ether "eth" or USDC "usdc".
const WARP_AMOUNT: u64 = 100;

const WARP_ROUTE_PROXY_ADDRESS: Address = usdc::WARP_ROUTE_PROXY;

const DANGO_RECIPIENT: Addr = addr!("a20a0e1a71b82d50fc046bc6e3178ad0154fd184");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let config = config::load_config()?;
    let evm_config = config.evm.get("sepolia").unwrap();

    let mut maybe_warp_route = None;
    for warp_route in evm_config.warp_routes.iter() {
        if warp_route.proxy_address == Some(WARP_ROUTE_PROXY_ADDRESS) {
            maybe_warp_route = Some(warp_route.clone());
            break;
        }
    }
    let warp_route = maybe_warp_route.ok_or(anyhow::anyhow!("Warp route not found in config"))?;
    let warp_route_proxy_address = warp_route.proxy_address.unwrap();

    let (dango_client, ..) = setup::setup_dango(&config.dango).await?;
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

    let hyperlane_protocol_fee = U256::from(evm_config.hyperlane_protocol_fee);

    // Setup ethereum provider
    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let value = match warp_route.warp_route_type {
        WarpRouteType::ERC20Collateral(erc20_address) => {
            println!(
                "approving spend of {} for route proxy ({}) on {}",
                WARP_AMOUNT,
                warp_route_proxy_address.to_string(),
                erc20_address.to_string()
            );
            let tx_hash = provider
                .send_transaction(
                    TransactionRequest::default()
                        .with_to(erc20_address)
                        .with_call(&HypERC20::approveCall {
                            spender: warp_route_proxy_address,
                            amount: U256::from(WARP_AMOUNT),
                        }),
                )
                .await?
                .watch()
                .await?;
            println!("done! tx hash: {tx_hash}");
            hyperlane_protocol_fee
        },
        WarpRouteType::Native => U256::from(WARP_AMOUNT) + hyperlane_protocol_fee,
    };

    // Setup contracts
    let warp_route_proxy = TokenRouter::new(warp_route.proxy_address.unwrap(), &provider);

    // Assert that the dango domain is correctly enrolled in the warp route proxy
    let router_address = warp_route_proxy.routers(dango_domain).call().await?;
    assert_eq!(
        router_address,
        FixedBytes::<32>::left_padding_from(app_cfg.addresses.warp.inner())
    );

    println!("warping {} {} to dango...", WARP_AMOUNT, &warp_route.symbol);
    let tx_hash = warp_route_proxy
        .transferRemote(
            dango_domain,
            FixedBytes::<32>::left_padding_from(DANGO_RECIPIENT.inner()),
            U256::from(WARP_AMOUNT),
        )
        .value(value)
        .send()
        .await?
        .watch()
        .await?;

    println!("done! tx hash: {tx_hash}");

    Ok(())
}
