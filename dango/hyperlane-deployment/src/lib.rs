pub mod addresses {
    pub mod sepolia {
        use alloy::primitives::{Address, address};

        pub const WARP_DOMAIN: u32 = 11155111;
        pub const HYPERLANE_MAILBOX: Address = address!("fFAEF09B3cd11D9b20d1a19bECca54EEC2884766");

        pub const HYPERLANE_STATIC_MESSAGE_ID_MULTISIG_ISM_FACTORY: Address =
            address!("FEb9585b2f948c1eD74034205a7439261a9d27DD");

        pub mod hyperlane_deployments {
            use super::*;

            pub const CUSTOM_MULTISIG_ISM: Address =
                address!("08A587C17C1CD3a1BC2220E0808281a143877B70");

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

    pub mod mailbox {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            Mailbox,
            "artifacts/evm/Mailbox.json"
        }
    }

    pub mod ism {
        use alloy::sol;

        sol! {
            #[sol(rpc)]
            IInterchainSecurityModule,
            "artifacts/evm/IInterchainSecurityModule.json"
        }

        sol! {
            #[sol(rpc)]
            IRoutingIsm,
            "artifacts/evm/IRoutingIsm.json"
        }

        sol! {
            #[sol(rpc)]
            DefaultFallbackRoutingIsm,
            "artifacts/evm/DefaultFallbackRoutingIsm.json"
        }

        sol! {
            #[sol(rpc)]
            StaticAggregationIsm,
            "artifacts/evm/StaticAggregationIsm.json"
        }

        sol! {
            #[sol(rpc)]
            AbstractMetaProxyMultisigIsm,
            "artifacts/evm/AbstractMetaProxyMultisigIsm.json"
        }

        sol! {
            #[sol(rpc)]
            StaticMessageIdMultisigIsm,
            "artifacts/evm/StaticMessageIdMultisigIsm.json"
        }

        sol! {
            #[sol(rpc)]
            StaticMessageIdMultisigIsmFactory,
            "artifacts/evm/StaticMessageIdMultisigIsmFactory.json"
        }

        sol! {
            #[sol(rpc)]
            StaticThresholdAddressSetFactory,
            "artifacts/evm/StaticThresholdAddressSetFactory.json"
        }

        sol! {
            #[sol(rpc)]
            TokenRouter,
            "artifacts/evm/TokenRouter.json"
        }
    }
}

pub mod config;
pub mod setup;

pub mod dango;
pub mod evm;
