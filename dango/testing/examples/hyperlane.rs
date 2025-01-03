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
        primitives::{address, bytes, fixed_bytes, Address, FixedBytes, U256},
        providers::{Provider, ProviderBuilder},
        rpc::types::TransactionRequest,
        signers::local::{coins_bip39::English, MnemonicBuilder},
        sol_types::SolCall,
    },
    dotenvy::dotenv,
    hyperlane_types::mailbox::Domain,
    std::env,
};

/// Mailbox contract on Sepolia.
const SEPOLIA_MAILBOX: Address = address!("fFAEF09B3cd11D9b20d1a19bECca54EEC2884766");

/// The required hook on Sepolia is set to the `ProtocolFee` contract which
/// charges a static fee of 1 wei for all outgoing transfers.
const SEPOLIA_PROTOCOL_FEE: U256 = U256::from_le_slice(&[1]);

/// We haven't registered Dango's domain in Hyperlane's registry yet.
/// For this demo just use a random number as a mockup.
const MOCK_DANGO_DOMAIN: Domain = 88888888;

/// The Warp contract on Dango which will handle the message.
const DANGO_WARP: FixedBytes<32> =
    fixed_bytes!("0000000000000000000000006c7bb6ed728a83469f57afa1000ca7ecd67652c3");

/// A spot account on Dango which will receive the ETH.
const DANGO_TOKEN_RECIPIENT: FixedBytes<32> =
    fixed_bytes!("000000000000000000000000c95bd5bfc20091c6383fcee88493c9df33eeaaaf");

mod hyp_erc20 {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        HypERC20,
        "testdata/HypERC20.json"
    }
}

mod hyp_erc20_collateral_metadata {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        HypERC20Collateral,
        "testdata/HypERC20CollateralMetadata.json"
    }
}

mod hyp_native_metadata {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        HypNativeMetadata,
        "testdata/HypNativeMetadata.json"
    }
}

mod proxy {
    use alloy::sol;

    sol! {
        #[sol(rpc)]
        ProxyAdmin,
        "testdata/ProxyAdmin.json"
    }

    sol! {
        #[sol(rpc)]
        TransparentUpgradeableProxy,
        "testdata/TransparentUpgradeableProxy.json"
    }
}

#[tokio::main]
async fn main() {
    dotenv().unwrap();

    let infura_api_key = env::var("INFURA_API_KEY").unwrap();
    let url = format!("https://sepolia.infura.io/v3/{infura_api_key}");

    let mnemonic = env::var("MNEMONIC").unwrap();
    let signer = MnemonicBuilder::<English>::default()
        .phrase(&mnemonic)
        .build()
        .unwrap();

    let owner = signer.address();
    println!("using {owner} as owner address");

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::new(signer))
        .on_http(url.parse().unwrap());

    println!("deploying ProxyAdmin contract...");
    let admin = proxy::ProxyAdmin::deploy(&provider, owner).await.unwrap();
    println!("done! address: {}", admin.address());

    println!("deploying HypNativeMetadata logic contract...");
    let native_logic = hyp_native_metadata::HypNativeMetadata::deploy(&provider, SEPOLIA_MAILBOX)
        .await
        .unwrap();
    println!("done! address: {}", native_logic.address());

    println!("deploying HypNativeMetadata proxy contract...");
    let native_proxy = proxy::TransparentUpgradeableProxy::deploy(
        &provider,
        *native_logic.address(),
        *admin.address(),
        hyp_native_metadata::HypNativeMetadata::initializeCall {
            _hook: Address::ZERO,                     // use mailbox default
            _interchainSecurityModule: Address::ZERO, // use mailbox default
            _owner: owner,
        }
        .abi_encode()
        .into(),
    )
    .await
    .unwrap();
    println!("done! address: {}", native_proxy.address());

    println!("enrolling dango domain in router...");
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*native_proxy.address())
                .with_call(
                    &hyp_native_metadata::HypNativeMetadata::enrollRemoteRouterCall {
                        _domain: MOCK_DANGO_DOMAIN,
                        _router: DANGO_WARP,
                    },
                ),
        )
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    println!("done! tx hash: {tx_hash}");

    println!("warping 100 wei...");
    let amount = U256::from(100);
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*native_proxy.address())
                .with_value(amount + SEPOLIA_PROTOCOL_FEE)
                .with_call(
                    &hyp_native_metadata::HypNativeMetadata::transferRemote_0Call {
                        _destination: MOCK_DANGO_DOMAIN,
                        _recipient: DANGO_TOKEN_RECIPIENT,
                        _amount: amount,
                        _tokenMetadata: bytes!("1234abcd"),
                    },
                ),
        )
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    println!("done! tx hash: {tx_hash}");
}
