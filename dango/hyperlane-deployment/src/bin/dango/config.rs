use std::fs;

use grug::Addr;

use {dango_types::gateway::Origin, serde::Serialize};

use {dango_types::gateway::Remote, serde::Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub origin: Origin,
    pub remote: Remote,
}

#[derive(Deserialize)]
pub struct Config {
    pub dango_api_url: String,
    pub dango_chain_id: String,
    pub routes: Vec<Route>,
}

pub fn load_config() -> anyhow::Result<Config> {
    let config_path = format!("{}/src/bin/dango/config.json", env!("CARGO_MANIFEST_DIR"));
    println!("loading config from: {}", config_path);
    let config = fs::read_to_string(config_path)?;
    println!("config: {}", config);
    let config: Config = serde_json::from_str(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use {
        dango_hyperlane_deployment::addresses::sepolia::hyperlane_deployments::usdc, grug::addr,
        hex_literal::hex, hyperlane_types::Addr32,
    };

    use super::*;

    #[test]
    fn test_load_config() {
        let config = load_config().unwrap();
    }

    #[test]
    fn t2() {
        let a = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");

        // serialize the address
        let serialized = serde_json::to_string(&a).unwrap();
        println!("serialized = {}", serialized);

        // deserialize the address
        let deserialized: Addr = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {}", deserialized);

        let addr32 = Addr32::from(a);
        let serialized = serde_json::to_string(&addr32).unwrap();
        println!("serialized = {}", serialized);

        // deserialize the address
        let deserialized: Addr32 = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {}", deserialized);
    }

    #[test]
    fn t3() {
        let h = hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

        // serialize the hex
        let serialized = serde_json::to_string(&h).unwrap();
        println!("serialized = {}", serialized);

        // deserialize the hex
        let deserialized: [u8; 32] = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {:?}", deserialized);
    }

    #[test]
    fn t4() {
        let x = usdc::WARP_ROUTE_PROXY;

        // serialize the address
        let serialized = serde_json::to_string(&x).unwrap();
        println!("serialized = {}", serialized);

        // deserialize the address
        let deserialized: Addr32 = serde_json::from_str(&serialized).unwrap();
        println!("deserialized = {}", deserialized);
    }
}
