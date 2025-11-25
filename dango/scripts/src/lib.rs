pub mod addresses {
    pub mod sepolia {
        pub mod hyperlane_deployments {
            use alloy::primitives::{Address, address};

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
            use alloy::primitives::{Address, address};

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
            "../../dependencies/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20Collateral.sol/HypERC20Collateral.json"
        }
    }

    pub mod hyp_erc20 {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            HypERC20,
            "../../dependencies/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/HypERC20.json"
        }
    }

    pub mod hyp_native {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            HypNative,
            "../../dependencies/hyperlane-monorepo/solidity/artifacts/contracts/token/HypNative.sol/HypNative.json"
        }
    }

    pub mod proxy {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            ProxyAdmin,
            "../../dependencies/hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/ProxyAdmin.json"
        }

        sol! {
            #[sol(rpc)]
            TransparentUpgradeableProxy,
            "../../dependencies/hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/TransparentUpgradeableProxy.json"
        }
    }
}
