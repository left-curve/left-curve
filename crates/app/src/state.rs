use cw_std::{Account, Addr, Binary, BlockInfo, Config, Hash, Item, Map};

/// A string that identifies the chain
pub const CHAIN_ID: Item<String> = Item::new("chain_id");

/// Chain-level configuration
pub const CONFIG: Item<Config> = Item::new("config");

/// The most recently finalized block
pub const LAST_FINALIZED_BLOCK: Item<BlockInfo> = Item::new("last_finalized_block");

/// Wasm contract byte codes: code_hash => byte_code
pub const CODES: Map<&Hash, Binary> = Map::new("code");

/// Account metadata: address => account
pub const ACCOUNTS: Map<&Addr, Account> = Map::new("account");

/// Each contract has its own storage space, which we term the "substore".
/// A key in a contract's substore is prefixed by the word "wasm" + contract address.
pub const CONTRACT_NAMESPACE: &[u8] = b"wasm";
