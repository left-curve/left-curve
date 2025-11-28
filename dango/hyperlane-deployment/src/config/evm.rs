use {
    alloy::primitives::Address,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct EVMConfig {
    pub infura_rpc_url: String,
    pub hyperlane_deployments: HyperlaneDeployments,
    pub hyperlane_domain: u32,
    pub hyperlane_protocol_fee: u128,
    pub ism: ISM,
    #[serde(default)]
    pub proxy_admin_address: Option<Address>,
    #[serde(default)]
    pub warp_routes: Vec<WarpRoute>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WarpRoute {
    pub warp_route_type: WarpRouteType,
    /// The address of the warp route contract. If set to Some the script
    /// will use the provided address. If set to None the script will deploy
    /// a new warp route contract.
    pub address: Option<Address>,
    /// The address of the proxy contract. If set to Some the script
    /// will use the provided address. If set to None the script will deploy
    /// a new proxy contract.
    pub proxy_address: Option<Address>,
    /// The symbol to use as subdenom for the token on Dango.
    pub symbol: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarpRouteType {
    #[serde(rename = "erc20_collateral")]
    ERC20Collateral(Address),
    Native,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ISM {
    StaticMessageIdMultisigIsm {
        validators: Vec<Address>,
        threshold: u8,
    },
}

impl EVMConfig {
    pub fn get_multisig_ism_factory_address(&self) -> Address {
        match self.ism {
            ISM::StaticMessageIdMultisigIsm { .. } => {
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
