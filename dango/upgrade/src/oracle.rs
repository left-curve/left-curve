// This file contains only the boilerplate.
#![allow(dead_code)]

use {
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    grug_types::{Addr, Storage, addr},
};

/// Address of the Oracle contract. Same on mainnet and testnet.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

/// Pre-migration oracle storage shapes.
mod legacy_oracle {
    // Add content here.
}

pub fn do_oracle_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut _oracle_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &ORACLE]);

    // Add migration logic here.

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    // Add tests here.
}
