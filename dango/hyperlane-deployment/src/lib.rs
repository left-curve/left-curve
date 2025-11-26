pub mod addresses {
    pub mod sepolia {
        use alloy::primitives::{Address, address};

        pub const WARP_DOMAIN: u32 = 11155111;
        pub const HYPERLANE_MAILBOX: Address = address!("fFAEF09B3cd11D9b20d1a19bECca54EEC2884766");

        pub mod hyperlane_deployments {
            use super::*;

            pub mod eth {
                use super::*;

                pub const PROXY_ADMIN: Address =
                    address!("0x947303E34C1a2B97fB00C68C1cC4cA97B3361fE6");
                pub const WARP_ROUTE: Address =
                    address!("0x613942EFf27c6886bb2a33A172CDaf03A009E601");
                pub const WARP_ROUTE_PROXY: Address =
                    address!("0x34DC3F292fC04e3Dcc2830AC69bb5d4cd5E8F654");
            }
            pub mod usdc {
                use super::*;

                pub const PROXY_ADMIN: Address =
                    address!("0xE071653043828C9923c79B04B077358D94Fc84f9");
                pub const WARP_ROUTE: Address =
                    address!("0xd05909852aE07118857f9D071781671D12c0f36c");
                pub const WARP_ROUTE_PROXY: Address =
                    address!("0x6BA100453E826b903De1a7AEcDb7A3396670aE51");
            }
        }

        pub mod erc20s {
            use super::*;

            pub const USDC: Address = address!("0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238");
        }
    }
}

pub mod contract_bindings {
    use alloy::sol;

    pub mod hyp_erc20_collateral {
        use super::*;

        sol! {
            #[sol(rpc)]
            HypERC20Collateral,
            "artifacts/evm/HypERC20Collateral.json"
        }
    }

    pub mod hyp_erc20 {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            HypERC20,
            "artifacts/evm/HypERC20.json"
        }
    }

    pub mod hyp_native {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            HypNative,
            "artifacts/evm/HypNative.json"
        }
    }

    pub mod proxy {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            ProxyAdmin,
            "artifacts/evm/ProxyAdmin.json"
        }

        sol! {
            #[sol(rpc)]
            TransparentUpgradeableProxy,
            "artifacts/evm/TransparentUpgradeableProxy.json"
        }
    }
}

pub mod config;
pub mod setup;
