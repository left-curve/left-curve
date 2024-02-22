use {
    crate::{Addr, Hash, Message, Timestamp, Uint64},
    hex_literal::hex,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeSet,
};

/// Genesis messages don't have senders, so we use this mock up hash as the
/// sender address. It is the SHA-256 hash of the UTF-8 string `sender`.
pub const GENESIS_SENDER: Addr = Addr(Hash(hex!("0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a")));

/// During genesis there isn't a block hash, so we use this mock up hash as the
/// block hash. It is the SHA-256 hash of the UTF-8 string `hash`.
pub const GENESIS_BLOCK_HASH: Hash = Hash(hex!("d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa"));

/// Chain-level configurations. Not to be confused with contract-level configs.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The account that can update this config. Typically it's recommended to
    /// set this to a decentralized governance contract. Setting this to None
    /// makes the config immutable.
    ///
    /// We name this `owner` instead of `admin` to avoid confusion with the
    /// contract admin, which is the account that can update a contract's `code_hash`.
    pub owner: Option<Addr>,
    /// A contract the manages fungible token transfers.
    ///
    /// Non-fungible tokens (NFTs) can be managed by this contract as well,
    /// using an approach similar to Solana's Metaplex standard:
    /// https://twitter.com/octalmage/status/1695165358955487426
    pub bank: Addr,
    /// A list of contracts that will be called at the beginning of each block,
    /// before any transaction, in order. Each of them must implement the `before_block`
    /// entry point.
    pub begin_blockers: Vec<Addr>,
    /// A list of contracts that will be called at the end of each block, after
    /// all transactions have been processed, in order. Each of them must
    /// implement the `after_block` entry point.
    pub end_blockers: Vec<Addr>,
    /// Which accounts are allowed to upload Wasm byte codes.
    pub store_code_permission: Permission,
    /// Which accounts are allowed to instantiate contracts.
    pub instantiate_permission: Permission,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Any account is allowed to perform the action
    Everybody,
    /// Only the owner can perform the action. Note, the owner is always able to
    /// upload code or instantiate contracts.
    Nobody,
    /// Accounts in a whitelist or the owner can perform the action.
    Accounts(BTreeSet<Addr>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GenesisState {
    pub config: Config,
    pub msgs:   Vec<Message>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockInfo {
    pub height:    Uint64,
    pub timestamp: Timestamp,
    pub hash:      Hash,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub code_hash: Hash,
    pub admin:     Option<Addr>,
}
