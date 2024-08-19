use {
    grug_storage::{Item, Map, Serde, Set},
    grug_types::{Account, Addr, Binary, BlockInfo, Config, Hash256, Json, Timestamp},
};

/// A string that identifies the chain
pub const CHAIN_ID: Item<String> = Item::new("chain_id");

/// Chain-level configuration
pub const CONFIG: Item<Config> = Item::new("config");

/// Application-specific configurations.
///
/// Note: This uses the JSON encoding, because `serde_json::Value` doesn't have
/// borsh traits derived.
pub const APP_CONFIGS: Map<&str, Json, Serde> = Map::new("app_config");

/// The most recently finalized block
pub const LAST_FINALIZED_BLOCK: Item<BlockInfo> = Item::new("last_finalized_block");

/// Scheduled cronjobs.
///
/// This needs to be a `Set` instead of `Map<Timestamp, Addr>` because there can
/// be multiple jobs with the same scheduled time.
pub const NEXT_CRONJOBS: Set<(Timestamp, Addr)> = Set::new("jobs");

/// Wasm contract byte codes: code_hash => byte_code
pub const CODES: Map<Hash256, Binary> = Map::new("code");

/// Account metadata: address => account
pub const ACCOUNTS: Map<Addr, Account> = Map::new("account");

/// Each contract has its own storage space, which we term the "substore".
/// A key in a contract's substore is prefixed by the word "wasm" + contract address.
pub const CONTRACT_NAMESPACE: &[u8] = b"wasm";
