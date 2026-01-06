use {
    alloy::primitives::Address,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct EVMConfig {
    pub chain_id: u32,
    pub infura_rpc_url: String,
    pub hyperlane_deployments: HyperlaneDeployments,
    pub hyperlane_domain: u32,
    pub hyperlane_protocol_fee: u128,
    pub ism: Ism,
    pub warp_routes: Vec<WarpRoute>,
    /// The address of the multi-sig that will be the new owner of the ProxyAdmin contract.
    pub multi_sig_address: Option<Address>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WarpRoute {
    pub warp_route_type: WarpRouteType,
    /// The symbol to use as subdenom for the token on Dango.
    pub symbol: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WarpRouteType {
    #[serde(rename = "erc20_collateral")]
    ERC20Collateral(Address),
    Native,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ism {
    StaticMessageIdMultisigIsm {
        validators: Vec<Address>,
        threshold: u8,
    },
}

impl EVMConfig {
    pub fn get_multisig_ism_factory_address(&self) -> Address {
        match self.ism {
            Ism::StaticMessageIdMultisigIsm { .. } => {
                self.hyperlane_deployments
                    .static_message_id_multisig_ism_factory
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HyperlaneDeployments {
    pub static_message_id_multisig_ism_factory: Address,
    pub mailbox: Address,
}
