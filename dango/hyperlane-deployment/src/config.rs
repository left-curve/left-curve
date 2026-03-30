pub mod dango;
pub mod evm;

use {
    crate::config::{
        dango::DangoConfig,
        evm::{EVMConfig, WarpRouteType},
    },
    alloy::primitives::Address,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub dango: DangoConfig,
    pub evm: EVMConfig,
}

pub fn load_config_from_path(config_path: &str) -> anyhow::Result<Config> {
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config)?;

    Ok(config)
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Deployments {
    pub evm: EVMDeployment,
}

pub fn load_deployments_from_path(deployments_path: &str) -> anyhow::Result<Deployments> {
    let deployments = std::fs::read_to_string(deployments_path)?;
    let deployments: Deployments = serde_json::from_str(&deployments)?;

    Ok(deployments)
}

pub fn save_deployments_to_path(
    deployments: &Deployments,
    deployments_path: &str,
) -> anyhow::Result<()> {
    let deployments_json = serde_json::to_string_pretty(deployments)?;
    std::fs::write(deployments_path, deployments_json)?;
    println!("Saved deployments to {deployments_path}");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        alloy::primitives::{Address, address},
    };

    #[test]
    fn test_load_config() {
        let config_path = format!("{}/config.ethereum.json", env!("CARGO_MANIFEST_DIR"));
        load_config_from_path(&config_path).unwrap();
    }

    #[test]
    fn t1() {
        let addr = address!("0xFEb9585b2f948c1eD74034205a7439261a9d27DD");

        let serialized = serde_json::to_string(&addr).unwrap();
        println!("serialized = {serialized}");

        let deserialized: Address = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {deserialized}");
    }

    #[test]
    fn test_load_deployments() {
        let deployments_path = format!("{}/deployments.ethereum.json", env!("CARGO_MANIFEST_DIR"));
        let deployments = load_deployments_from_path(&deployments_path).unwrap();
        println!("deployments = {deployments:?}");
    }
}
