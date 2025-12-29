use {
    dango_types::bitcoin::{Config as BitcoinBridgeConfig, MultisigSettings, Network},
    sea_orm::DatabaseConnection,
};

#[derive(Clone)]
pub struct Context {
    /// The bitcoin bridge multisig settings.
    pub multisig_settings: MultisigSettings,
    /// The Bitcoin network the bridge is operating on.
    pub network: Network,
    /// The database connection.
    pub db: DatabaseConnection,
}

impl Context {
    pub fn new(bridge_config: BitcoinBridgeConfig, db: DatabaseConnection) -> Self {
        Self {
            multisig_settings: bridge_config.multisig,
            network: bridge_config.network,
            db,
        }
    }
}
