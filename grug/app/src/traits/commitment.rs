use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_storage::Item,
    grug_types::{Batch, Hash256, Op, StdResult, Storage},
    sha2::{Digest, Sha256},
};

/// Represents a state commitment scheme.
///
/// A commitment scheme generates a fixed-length root hash for a state of
/// arbitrary size. The root hash is used in two ways:
///
/// 1. For consensus. If two nodes see they have the same root hash, they are
///    sure they have the same state, without having to check the state itself.
/// 2. For light clients. The commitment scheme should be able to generate proof
///    that a given key-value pair exists or does not exits in the state. This
///    can be used in light clients, which has application in trust-minimized
///    cross-chain bridging.
pub trait Commitment {
    type Proof: BorshSerialize + BorshDeserialize;

    fn root_hash(storage: &dyn Storage, version: u64) -> StdResult<Option<Hash256>>;

    fn apply(
        storage: &mut dyn Storage,
        old_version: u64,
        new_version: u64,
        batch: &Batch,
    ) -> StdResult<Option<Hash256>>;

    fn prove(storage: &dyn Storage, key_hash: Hash256, version: u64) -> StdResult<Self::Proof>;

    fn prune(storage: &mut dyn Storage, up_to_version: u64) -> StdResult<()>;
}

const LATEST_VERSION_AND_ROOT_HASH: Item<'static, (u64, Hash256)> = Item::new("latest");

/// The simplest possible commitment scheme. It simply hashes the changeset.
pub struct SimpleCommitment;

impl Commitment for SimpleCommitment {
    type Proof = ();

    fn root_hash(storage: &dyn Storage, version: u64) -> StdResult<Option<Hash256>> {
        // If a latest version exists the it equals the version requested, then
        // return the stored root hash. Otherwise return `None`.
        match LATEST_VERSION_AND_ROOT_HASH.may_load(storage)? {
            Some((latest_version, root_hash)) if latest_version == version => Ok(Some(root_hash)),
            _ => Ok(None),
        }
    }

    fn apply(
        storage: &mut dyn Storage,
        old_version: u64,
        new_version: u64,
        batch: &Batch,
    ) -> StdResult<Option<Hash256>> {
        debug_assert!(
            new_version == 0 || new_version > old_version,
            "version is not incremental"
        );

        let mut hasher = Sha256::new();
        for (k, op) in batch {
            hasher.update((k.len() as u16).to_be_bytes());
            hasher.update(k);
            if let Op::Insert(v) = op {
                hasher.update([1]);
                hasher.update((v.len() as u16).to_be_bytes());
                hasher.update(v);
            } else {
                hasher.update([0]);
            }
        }
        let root_hash = Hash256::from_inner(hasher.finalize().into());

        LATEST_VERSION_AND_ROOT_HASH.save(storage, &(new_version, root_hash))?;

        Ok(Some(root_hash))
    }

    fn prove(_storage: &dyn Storage, _key_hash: Hash256, _version: u64) -> StdResult<Self::Proof> {
        Ok(())
    }

    fn prune(_storage: &mut dyn Storage, _up_to_version: u64) -> StdResult<()> {
        Ok(())
    }
}
