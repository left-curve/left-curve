use std::collections::BTreeMap;

use {
    alloy::primitives::Address,
    serde::{Deserialize, Serialize},
};

use crate::config::{
    dango::DangoConfig,
    evm::{EVMConfig, WarpRouteType},
};

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

    Ok(config)
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let config_path = format!("{}/config.json", env!("CARGO_MANIFEST_DIR"));
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, config_json)?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EVMDeployment {
    pub proxy_admin_address: Address,
    pub warp_routes: Vec<(WarpRouteType, EVMWarpRouteDeployment)>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct EVMWarpRouteDeployment {
    pub address: Address,
    pub proxy_address: Address,
    pub symbol: String,
}

pub fn load_evm_deployments() -> anyhow::Result<EVMDeployment> {
    let deployments_path = format!("{}/deployments.json", env!("CARGO_MANIFEST_DIR"));
    let deployments = std::fs::read_to_string(deployments_path)?;
    let deployments: EVMDeployment = serde_json::from_str(&deployments)?;

    Ok(deployments)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Deployments {
    pub evm: BTreeMap<String, EVMDeployment>,
}

pub fn load_deployments() -> anyhow::Result<Deployments> {
    let deployments_path = format!("{}/deployments.json", env!("CARGO_MANIFEST_DIR"));
    let deployments = std::fs::read_to_string(deployments_path)?;
    let deployments: Deployments = serde_json::from_str(&deployments)?;

    Ok(deployments)
}

pub fn save_deployments(deployments: &Deployments) -> anyhow::Result<()> {
    let deployments_path = format!("{}/deployments.json", env!("CARGO_MANIFEST_DIR"));
    let deployments_json = serde_json::to_string_pretty(deployments)?;
    std::fs::write(deployments_path.clone(), deployments_json)?;
    println!("Saved deployments to {}", deployments_path);
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
    fn test_load_evm_deployments() {
        let deployments = load_evm_deployments().unwrap();
        println!("deployments = {:?}", deployments);
    }

    #[test]
    fn test_load_deployments() {
        let deployments = load_deployments().unwrap();
        println!("deployments = {:?}", deployments);
    }
}
