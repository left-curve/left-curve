use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    /// GraphQL endpoint of the Dango node.
    pub url: String,
    /// Chain ID used when signing transactions.
    pub chain_id: String,
    /// Multiplier applied to the simulated gas cost of a transaction.
    pub gas_adjustment: f64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:8080".to_string(),
            chain_id: "dango-1".to_string(),
            gas_adjustment: 1.4,
        }
    }
}
