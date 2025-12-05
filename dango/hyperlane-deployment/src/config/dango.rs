use {dango_types::gateway::Origin, serde::Serialize};

use {dango_types::gateway::Remote, serde::Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Route {
    pub origin: Origin,
    pub remote: Remote,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DangoConfig {
    pub api_url: String,
    pub chain_id: String,
}

#[cfg(test)]
mod tests {
    use {
        grug::{Addr, addr},
        hex_literal::hex,
        hyperlane_types::Addr32,
    };

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
}
