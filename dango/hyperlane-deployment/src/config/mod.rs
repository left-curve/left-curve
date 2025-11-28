use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::{dango::DangoConfig, evm::EVMConfig};

pub mod dango;
pub mod evm;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub dango: DangoConfig,
    pub evm: BTreeMap<String, EVMConfig>,
}

pub fn load_config() -> anyhow::Result<Config> {
    let config_path = format!("{}/config.json", env!("CARGO_MANIFEST_DIR"));
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config)?;

    // Validate the config
    for warp_route in config
        .evm
        .iter()
        .flat_map(|(_, evm_config)| evm_config.warp_routes.iter())
    {
        if warp_route.address.is_none() != warp_route.proxy_address.is_none() {
            return Err(anyhow::anyhow!(
                "warp_route.address and warp_route.proxy_address must be either both set or both unset"
            ));
        }
    }

    Ok(config)
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let config_path = format!("{}/config.json", env!("CARGO_MANIFEST_DIR"));
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, config_json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        crate::config::evm::{HyperlaneDeployments, ISM, WarpRoute, WarpRouteType},
        alloy::primitives::{Address, address},
        grug::btree_map,
    };

    use super::*;

    #[test]
    fn test_load_config() {
        let config = load_config().unwrap();
    }

    #[test]
    fn t1() {
        let addr = address!("0xFEb9585b2f948c1eD74034205a7439261a9d27DD");
        let serialized = serde_json::to_string(&addr).unwrap();
        println!("serialized = {}", serialized);
        let deserialized: Address = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {}", deserialized);
    }

    #[test]
    fn t2() {
        let b = btree_map! {
            "sepolia" => EVMConfig {
                infura_rpc_url: "https://sepolia.infura.io/v3/".to_string(),
                hyperlane_deployments: HyperlaneDeployments {
                    static_message_id_multisig_ism_factory: address!("0xFEb9585b2f948c1eD74034205a7439261a9d27DD"),
                    mailbox: address!("0xFEb9585b2f948c1eD74034205a7439261a9d27DD"),
                },
                hyperlane_domain: 11155111,
                hyperlane_protocol_fee: 1,
                ism: ISM::StaticMessageIdMultisigIsm {
                    validators: vec![address!("0x4e5088dd05269194c9cdf30cd7a72a2ddd31b23c")],
                    threshold: 1,
                },
                proxy_admin_address: Some(address!("0x311d8cd0eddab142be43a7f794b9013408675dbb")),
                warp_routes: vec![
                    WarpRoute {
                        warp_route_type: WarpRouteType::Native,
                        address: Some(address!("0x613942eff27c6886bb2a33a172cdaf03a009e601")),
                        proxy_address: Some(address!("0x34dc3f292fc04e3dcc2830ac69bb5d4cd5e8f654")),
                        symbol: "sepoliaETH".to_string(),
                    },
                ],
            },
        };
        let serialized = serde_json::to_string_pretty(&b).unwrap();
        println!("serialized = {}", serialized);
    }
}
