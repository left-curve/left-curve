use {
    crate::{Addr, Duration, Hash256, Json, Label, Message, Timestamp, Tx},
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
/// This is the RIPEMD-160 hash of the UTF-8 string `"sender"`.
pub const GENESIS_SENDER: Addr = Addr::from_inner(hex!("114af6e7a822df07328fba90e1546b5c2b00703f"));

/// The mock up block hash used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This is the SHA-256 hash of the UTF-8 string `"hash"`.
pub const GENESIS_BLOCK_HASH: Hash256 = Hash256::from_inner(hex!(
    "d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa"
));

/// The mock up block height used for executing genesis messages.
///
/// Genesis isn't part of a block, so there isn't actually a block hash. We use
/// this as a mock up.
///
/// This has to be zero, such as subsequent block heights are the same as the
/// database and Merkle tree version.
pub const GENESIS_BLOCK_HEIGHT: u64 = 0;

/// The chain's genesis state. To be included in the `app_state` field of
/// CometBFT's `genesis.json`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenesisState {
    /// Chain configurations.
    pub config: Config,
    /// App-specific configurations.
    pub app_config: Json,
    /// Messages to be executed in order during genesis.
    pub msgs: Vec<Message>,
}

/// Chain-level configurations. Not to be confused with contract-level configs.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The account that can update this config.
    pub owner: Addr,
    /// The contract the manages fungible token transfers.
    pub bank: Addr,
    /// The contract that handles transaction fees.
    pub taxman: Addr,
    /// A list of contracts that are to be called at regular time intervals.
    pub cronjobs: BTreeMap<Addr, Duration>,
    /// Permissions for certain gated actions.
    pub permissions: Permissions,
    /// Maximum age allowed for orphaned codes.
    /// A code is deleted if it remains orphaned (not used by any contract) for
    /// longer than this duration.
    pub max_orphan_age: Duration,
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

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(deny_unknown_fields)]
pub struct BlockInfo {
    pub height: u64,
    pub timestamp: Timestamp,
    pub hash: Hash256,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub info: BlockInfo,
    pub txs: Vec<(Tx, Hash256)>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractInfo {
    pub code_hash: Hash256,
    pub label: Option<Label>,
    pub admin: Option<Addr>,
}
