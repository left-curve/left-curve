//! Frozen pre-0.26.0 ("wire v1") shapes of the transaction path, kept solely
//! for **reading** historical Borsh-serialized blocks.
//!
//! 0.26.0 (#2192) removed the taxman contract and changed [`Config`] in place
//! (dropped `taxman`, added `gas_token` / `gas_fee_rate` / `gas_exemptions`).
//! Borsh is positional — no field names, no tags — so any persisted block that
//! *contains* a [`Message::Configure`] embeds the old `Config` layout and no
//! longer deserializes with the current types. Exactly seven blocks do:
//!
//! | network | height   | date (2026) |
//! |---------|----------|-------------|
//! | mainnet | 16847666 | Apr 8       |
//! | mainnet | 18782883 | Apr 20      |
//! | mainnet | 18786698 | Apr 20      |
//! | mainnet | 18787415 | Apr 20      |
//! | testnet | 18563232 | Apr 1       |
//! | testnet | 23062422 | Apr 20      |
//! | testnet | 23063329 | Apr 20      |
//!
//! The event/outcome side needs no shadow: 0.26.0 deliberately retained the
//! affected outcome types ("retained so that historical, Borsh-serialized
//! cached blocks still deserialize" — see `EvtFinalize`, `EvtWithhold`). This
//! module extends the same policy to the one type that could not be retained,
//! because its new fields migrated out of the deleted taxman's storage.
//!
//! Rules for this module:
//!
//! - **Never modify these types.** They mirror the exact wire layout the seven
//!   blocks above were written with. Field order is Borsh layout.
//! - Variants of [`MessageV1`] must keep the **same order** as the era's
//!   `Message` (Borsh enums serialize the variant index).
//! - Serde derives mirror the era's attributes so a V1 block serializes to
//!   **exactly the JSON it always had** (`taxman` present, no gas fields) —
//!   history is served faithfully, hashes and signatures stay valid.
//! - Going forward, types embedded in persisted data are **append-only**: add
//!   a new message variant instead of changing an existing struct in place.

use {
    crate::{
        Addr, BlockInfo, BlockOutcome, Duration, FullBlock, Hash256, HttpRequestDetails, Json,
        MsgExecute, MsgInstantiate, MsgMigrate, MsgTransfer, MsgUpgrade, MsgUpload, NonEmpty,
        Permissions, Tx,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::{BTreeMap, HashMap},
};

/// Chain-level configurations as they existed before 0.26.0: `taxman` still
/// present, no `gas_token` / `gas_fee_rate` / `gas_exemptions`.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1 {
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
    pub max_orphan_age: Duration,
}

/// [`crate::MsgConfigure`] carrying the pre-0.26.0 [`ConfigV1`].
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MsgConfigureV1 {
    pub new_cfg: Option<ConfigV1>,
    pub new_app_cfg: Option<Json>,
}

/// [`crate::Message`] with the pre-0.26.0 `Configure` payload. Every other
/// variant reuses the current type — none of them changed wire layout. The
/// variant **order** mirrors the era's enum: Borsh serializes the index.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageV1 {
    Configure(MsgConfigureV1),
    Upgrade(MsgUpgrade),
    Transfer(MsgTransfer),
    Upload(MsgUpload),
    Instantiate(MsgInstantiate),
    Execute(MsgExecute),
    Migrate(MsgMigrate),
}

/// [`crate::Tx`] whose messages may carry the pre-0.26.0 `Configure`.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct TxV1 {
    pub sender: Addr,
    pub gas_limit: u64,
    pub msgs: NonEmpty<Vec<MessageV1>>,
    pub data: Json,
    pub credential: Json,
}

/// [`crate::Block`] in the pre-0.26.0 wire layout.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlockV1 {
    pub info: BlockInfo,
    pub txs: Vec<(TxV1, Hash256)>,
}

/// [`FullBlock`] in the pre-0.26.0 wire layout.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FullBlockV1 {
    pub block: BlockV1,
    pub outcome: BlockOutcome,
}

/// [`crate::BlockAndBlockOutcomeWithHttpDetails`] — the block-cache file
/// payload — in the pre-0.26.0 wire layout.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockAndBlockOutcomeWithHttpDetailsV1 {
    pub block: BlockV1,
    pub block_outcome: BlockOutcome,
    pub http_request_details: HashMap<Hash256, HttpRequestDetails>,
}

// ------------------------------- compat views --------------------------------

/// Raw block bytes decode as neither the current schema nor legacy v1.
#[derive(Debug, thiserror::Error)]
#[error(
    "block bytes decode as neither the current schema ({current}) nor the pre-0.26.0 v1 schema ({v1})"
)]
pub struct LegacyDecodeError {
    pub current: std::io::Error,
    pub v1: std::io::Error,
}

/// A block in whichever wire schema it was written. Serializes untagged, so a
/// V1 block renders **exactly the JSON it always had**.
///
/// Borsh derives give the compat enum a one-byte variant tag — use it for
/// *new* writes (e.g. the archive's block store); historical bare payloads are
/// handled by [`Self::decode_borsh`]'s fallback.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum FullBlockCompat {
    Current(FullBlock),
    V1(FullBlockV1),
}

/// A borrowed transaction from either schema. Serializes untagged — faithful
/// to its own era's JSON.
#[derive(Serialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum TxRefCompat<'a> {
    Current(&'a Tx),
    V1(&'a TxV1),
}

impl TxRefCompat<'_> {
    pub fn sender(&self) -> &Addr {
        match self {
            Self::Current(tx) => &tx.sender,
            Self::V1(tx) => &tx.sender,
        }
    }
}

impl FullBlockCompat {
    /// Decode a **bare** (untagged) borsh payload: try the current schema
    /// first — the overwhelming majority — then fall back to legacy v1.
    ///
    /// The whole-payload retry is deliberate: borsh is positional and streams
    /// don't rewind, so version detection cannot happen locally inside
    /// `Config`. `from_slice` requires consuming every byte, which makes an
    /// accidental cross-schema parse practically impossible.
    pub fn decode_borsh(bytes: &[u8]) -> Result<Self, LegacyDecodeError> {
        match borsh::from_slice::<FullBlock>(bytes) {
            Ok(block) => Ok(Self::Current(block)),
            Err(current) => match borsh::from_slice::<FullBlockV1>(bytes) {
                Ok(block) => Ok(Self::V1(block)),
                Err(v1) => Err(LegacyDecodeError { current, v1 }),
            },
        }
    }

    pub fn info(&self) -> &BlockInfo {
        match self {
            Self::Current(b) => &b.block.info,
            Self::V1(b) => &b.block.info,
        }
    }

    pub fn outcome(&self) -> &BlockOutcome {
        match self {
            Self::Current(b) => &b.outcome,
            Self::V1(b) => &b.outcome,
        }
    }

    pub fn tx_count(&self) -> usize {
        match self {
            Self::Current(b) => b.block.txs.len(),
            Self::V1(b) => b.block.txs.len(),
        }
    }

    pub fn tx(&self, idx: usize) -> Option<(TxRefCompat<'_>, &Hash256)> {
        match self {
            Self::Current(b) => b
                .block
                .txs
                .get(idx)
                .map(|(tx, hash)| (TxRefCompat::Current(tx), hash)),
            Self::V1(b) => b
                .block
                .txs
                .get(idx)
                .map(|(tx, hash)| (TxRefCompat::V1(tx), hash)),
        }
    }

    pub fn txs(&self) -> impl Iterator<Item = (TxRefCompat<'_>, &Hash256)> {
        (0..self.tx_count()).map(move |idx| self.tx(idx).expect("index within tx_count"))
    }
}

/// A block-cache file payload in whichever wire schema it was written.
/// Serializes untagged — a V1 file renders its era's exact JSON.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum CachedBlockCompat {
    Current(crate::BlockAndBlockOutcomeWithHttpDetails),
    V1(BlockAndBlockOutcomeWithHttpDetailsV1),
}

impl CachedBlockCompat {
    /// Decode a cache-file payload: current schema first, then legacy v1. See
    /// [`FullBlockCompat::decode_borsh`] for why the retry spans the whole
    /// payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, LegacyDecodeError> {
        match borsh::from_slice::<crate::BlockAndBlockOutcomeWithHttpDetails>(bytes) {
            Ok(data) => Ok(Self::Current(data)),
            Err(current) => match borsh::from_slice::<BlockAndBlockOutcomeWithHttpDetailsV1>(bytes)
            {
                Ok(data) => Ok(Self::V1(data)),
                Err(v1) => Err(LegacyDecodeError { current, v1 }),
            },
        }
    }

    /// Project to the `{block, outcome}` shape the httpd serves, dropping the
    /// `http_request_details` (client IPs) exactly like the current routes do.
    pub fn into_full_block(self) -> FullBlockCompat {
        match self {
            Self::Current(data) => FullBlockCompat::Current(FullBlock {
                block: data.block,
                outcome: data.block_outcome,
            }),
            Self::V1(data) => FullBlockCompat::V1(FullBlockV1 {
                block: data.block,
                outcome: data.block_outcome,
            }),
        }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{Block, BlockAndBlockOutcomeWithHttpDetails, Permission, Timestamp},
    };

    fn config_v1() -> ConfigV1 {
        ConfigV1 {
            owner: Addr::mock(1),
            bank: Addr::mock(2),
            taxman: Addr::mock(3),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
            },
            max_orphan_age: Duration::from_hours(1),
        }
    }

    /// A block shaped like the seven legacy ones: a single transaction whose
    /// only message is a pre-0.26.0 `Configure`.
    fn full_block_v1() -> FullBlockV1 {
        FullBlockV1 {
            block: BlockV1 {
                info: BlockInfo {
                    height: 16847666,
                    timestamp: Timestamp::from_nanos(0),
                    hash: Hash256::ZERO,
                },
                txs: vec![(
                    TxV1 {
                        sender: Addr::mock(9),
                        gas_limit: 1_000_000,
                        msgs: NonEmpty::new_unchecked(vec![MessageV1::Configure(MsgConfigureV1 {
                            new_cfg: Some(config_v1()),
                            new_app_cfg: None,
                        })]),
                        data: Json::from_inner(serde_json::Value::Null),
                        credential: Json::from_inner(serde_json::Value::Null),
                    },
                    Hash256::ZERO,
                )],
            },
            outcome: BlockOutcome {
                height: 16847666,
                app_hash: Hash256::ZERO,
                cron_outcomes: vec![],
                tx_outcomes: vec![],
            },
        }
    }

    fn full_block_current() -> FullBlock {
        FullBlock {
            block: Block {
                info: BlockInfo {
                    height: 42,
                    timestamp: Timestamp::from_nanos(0),
                    hash: Hash256::ZERO,
                },
                txs: vec![],
            },
            outcome: BlockOutcome {
                height: 42,
                app_hash: Hash256::ZERO,
                cron_outcomes: vec![],
                tx_outcomes: vec![],
            },
        }
    }

    /// Bare V1 borsh — the exact shape of the node's seven legacy cache
    /// blocks — is rejected by the current schema, picked up by the fallback,
    /// and rendered as its era's exact JSON (taxman present, no gas fields).
    #[test]
    fn bare_v1_bytes_decode_via_fallback_and_render_faithfully() {
        let bytes = borsh::to_vec(&full_block_v1()).unwrap();

        // The current schema must NOT accidentally accept these bytes — this
        // is the exact failure mode of the seven historical blocks.
        assert!(borsh::from_slice::<FullBlock>(&bytes).is_err());

        let compat = FullBlockCompat::decode_borsh(&bytes).unwrap();
        assert!(matches!(compat, FullBlockCompat::V1(_)));

        let json = serde_json::to_string(&compat).unwrap();
        assert!(json.contains(r#""taxman""#));
        assert!(!json.contains("gas_token"));
    }

    #[test]
    fn bare_current_bytes_decode_as_current() {
        let bytes = borsh::to_vec(&full_block_current()).unwrap();
        let compat = FullBlockCompat::decode_borsh(&bytes).unwrap();
        assert!(matches!(compat, FullBlockCompat::Current(_)));
    }

    /// The tagged compat enum — the archive store's on-disk format for new
    /// writes — roundtrips both variants.
    #[test]
    fn tagged_compat_roundtrips() {
        for compat in [
            FullBlockCompat::V1(full_block_v1()),
            FullBlockCompat::Current(full_block_current()),
        ] {
            let bytes = borsh::to_vec(&compat).unwrap();
            assert_eq!(
                borsh::from_slice::<FullBlockCompat>(&bytes).unwrap(),
                compat
            );
        }
    }

    /// The JSON wire — the archive's ingest path — deserializes untagged into
    /// the right variant for both eras.
    #[test]
    fn untagged_json_picks_the_right_variant() {
        let v1 = FullBlockCompat::V1(full_block_v1());
        let json = serde_json::to_string(&v1).unwrap();
        assert!(matches!(
            serde_json::from_str::<FullBlockCompat>(&json).unwrap(),
            FullBlockCompat::V1(_)
        ));

        let current = FullBlockCompat::Current(full_block_current());
        let json = serde_json::to_string(&current).unwrap();
        assert!(matches!(
            serde_json::from_str::<FullBlockCompat>(&json).unwrap(),
            FullBlockCompat::Current(_)
        ));
    }

    /// Cache-file payloads (block + outcome + http_request_details) go through
    /// the same fallback, and project to the served `{block, outcome}` shape.
    #[test]
    fn cached_block_decode_v1_and_project() {
        let v1 = full_block_v1();
        let cached = BlockAndBlockOutcomeWithHttpDetailsV1 {
            block: v1.block.clone(),
            block_outcome: v1.outcome.clone(),
            http_request_details: HashMap::new(),
        };
        let bytes = borsh::to_vec(&cached).unwrap();

        let compat = CachedBlockCompat::decode(&bytes).unwrap();
        assert!(matches!(compat, CachedBlockCompat::V1(_)));
        assert_eq!(compat.into_full_block(), FullBlockCompat::V1(v1));

        let current = full_block_current();
        let cached = BlockAndBlockOutcomeWithHttpDetails {
            block: current.block.clone(),
            block_outcome: current.outcome.clone(),
            http_request_details: HashMap::new(),
        };
        let bytes = borsh::to_vec(&cached).unwrap();
        assert!(matches!(
            CachedBlockCompat::decode(&bytes).unwrap(),
            CachedBlockCompat::Current(_)
        ));
    }
}
