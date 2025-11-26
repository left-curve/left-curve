use dango_types::bitcoin::{Config as BitcoinBridgeConfig, MultisigSettings, Network};

#[derive(Clone)]
pub struct Context {
    /// The bitcoin bridge multisig settings.
    pub multisig_settings: MultisigSettings,
    /// The Bitcoin network the bridge is operating on.
    pub network: Network,
}

impl Context {
    pub fn new(bridge_config: BitcoinBridgeConfig) -> Self {
        Self {
            multisig_settings: bridge_config.multisig,
            network: bridge_config.network,
        }
    }
}
