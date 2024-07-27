use {
    crate::{Addr, Duration, Event, GenericResult, Hash, Message, NumberConst, Timestamp, Uint64},
    borsh::{BorshDeserialize, BorshSerialize},
    hex_literal::hex,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::{BTreeMap, BTreeSet},
};

/// The mock up sender address used for executing genesis messages.
///
/// Genesis messages aren't sent by a transaction, so don't actually have sender.
/// We use this as a mock up.
///
/// This is the SHA-256 hash of the UTF-8 string `"sender"`.
pub const GENESIS_SENDER: Addr = Addr(Hash(hex!(
    "0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a"
)));

/// The mock up block hash used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This is the SHA-256 hash of the UTF-8 string `"hash"`.
pub const GENESIS_BLOCK_HASH: Hash = Hash(hex!(
    "d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa"
));

/// The mock up block height used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This has to be zero, such as subsequent block heights are the same as the
/// database and Merkle tree version.
pub const GENESIS_BLOCK_HEIGHT: Uint64 = Uint64::ZERO;

/// The chain's genesis state. To be included in the `app_state` field of
/// CometBFT's `genesis.json`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenesisState {
    pub config: Config,
    pub msgs: Vec<Message>,
}

/// Chain-level configurations. Not to be confused with contract-level configs.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The account that can update this config. Typically it's recommended to
    /// set this to a decentralized governance contract. Setting this to None
    /// makes the config immutable.
    ///
    /// We name this `owner` instead of `admin` to avoid confusion with the
    /// contract admin, which is the account that can update a contract's `code_hash`.
    pub owner: Option<Addr>,
    /// The contract the manages fungible token transfers.
    ///
    /// Non-fungible tokens (NFTs) can be managed by this contract as well,
    /// using an approach similar to Solana's Metaplex standard:
    /// <https://twitter.com/octalmage/status/1695165358955487426>
    pub bank: Addr,
    /// The contract that handles transaction fees.
    pub taxman: Addr,
    /// A list of contracts that are to be called at regular time intervals.
    pub cronjobs: BTreeMap<Addr, Duration>,
    /// Permissions for certain gated actions.
    pub permissions: Permissions,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Permissions {
    pub upload: Permission,
    pub instantiate: Permission,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Permission {
    /// Only the owner can perform the action. Note, the owner is always able to
    /// upload code or instantiate contracts.
    Nobody,
    /// Any account is allowed to perform the action
    Everybody,
    /// Some whitelisted accounts or the owner can perform the action.
    Somebodies(BTreeSet<Addr>),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlockInfo {
    pub height: Uint64,
    pub timestamp: Timestamp,
    pub hash: Hash,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub code_hash: Hash,
    pub admin: Option<Addr>,
}

/// Outcome of executing a single message, transaction, or cronjob.
///
/// Includes the events emitted, and gas consumption.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    // `None` means the call was done with unlimited gas, such as cronjobs.
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub result: GenericResult<Vec<Event>>,
}

/// Outcome of executing a block.
pub struct BlockOutcome {
    /// The Merkle root hash after executing this block.
    pub app_hash: Hash,
    /// Results of executing the cronjobs.
    pub cron_outcomes: Vec<Outcome>,
    /// Results of executing the transactions.
    pub tx_outcomes: Vec<Outcome>,
}
