// This file contains only the boilerplate.
#![allow(dead_code)]

use {
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    grug_types::{Addr, Storage, addr},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Pre-migration perps storage shapes.
mod legacy_perps {
    // Add content here.
}

pub fn do_perps_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    let mut _perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    // Add migration logic here.

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    // Add tests here.
}
