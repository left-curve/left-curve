use {
    crate::{GasTracker, QuerierProvider, StorageProvider},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Batch, Context, Hash256, QuerierWrapper, Storage},
    ics23::CommitmentProof,
    prost::bytes::Bytes,
};

// ------------------------------------ db -------------------------------------

/// Represents a database that our blockchain operates over.
///
/// The database should follow [ADR-065](https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md).
/// That is, it should contain two components:
/// - **state commitment**, a _Merklized_ KV store that stores _hashed_ keys and
///   _hashed_ values;
/// - **state storage**, a _flat_ KV store that stores _raw_ keys and _raw_ values.
///
/// The `state_commitment` and `state_storage` methods should return an _owned_
/// instance of the storage object (see the required `'static` lifetime). This
/// is required by the Wasm runtime. Additionally, storage object should be
/// _read only_. The host should write changes to an in-memory caching layer
/// (e.g. using `grug_db::Buffer`) and then use the `flush` and `commit`
/// methods to persist them.
///
/// The two mutable methods `flush` and `commit` take an immutable reference
/// of self (`&self`) instead of a mutable one. This is needed for multithreading.
/// For this reason, the implementation should use the [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
/// pattern such as `Arc<RwLock<T>>`.
///
/// We ship three implementations of `Db`:
/// - for use in production nodes, a physical DB based on RocksDB;
/// - for testing, a temporary, in-memory KV store;
/// - for tests that utilize mainnet data ("mainnet forking"), a DB that pulls
///   data from a remote archive node.
pub trait Db {
    type Error: ToString;

    /// A _flat_ KV store that stores _raw_ keys and _raw_ values.
    type StateStorage: Storage + Clone + 'static;

    /// A _Merklized_ KV store that stores _hashed_ keys and _hashed_ values.
    type StateCommitment: Storage + Clone + 'static;

    /// Type of the Merkle proof. The DB can choose any Merkle tree scheme.
    type Proof: BorshSerialize + BorshDeserialize;

    /// Return the state commitment.
    fn state_commitment(&self) -> Self::StateCommitment;

    /// Return the state storage.
    ///
    /// Error if the specified version has already been pruned.
    fn state_storage(&self, version: Option<u64>) -> Result<Self::StateStorage, Self::Error>;

    /// Return the most recent version that has been committed.
    ///
    /// `None` if not a single version has been committed.
    fn latest_version(&self) -> Option<u64>;

    /// Return the Merkle root hash at the specified version.
    ///
    /// If version is unspecified, return that of the latest committed version.
    ///
    /// `None` if the Merkle tree is empty at that version, or if that version
    /// has been pruned (we can't differentiate these two situations).
    fn root_hash(&self, version: Option<u64>) -> Result<Option<Hash256>, Self::Error>;

    /// Generate Merkle proof of the given key at the given version.
    ///
    /// If version is unspecified, use the latest version.
    ///
    /// If the key exists at that version, the returned value should be a Merkle
    /// _membership_ proof; otherwise, it should be a _non-membership_ proof.
    fn prove(&self, key: &[u8], version: Option<u64>) -> Result<Self::Proof, Self::Error>;

    /// Generate ICS-23 compatible Merkle proof of the given key at the given
    /// version.
    ///
    /// If version is unspecified, use the latest version.
    ///
    /// ## Note
    ///
    /// This needs to be implemented at the `Db` level, instead of in grug-jmt,
    /// because ICS-23 requires proofs to contain the prehash key and value,
    /// while grug-jmt only store hashed keys and values. Therefore we need the
    /// state storage.
    fn ics23_prove(
        &self,
        key: Vec<u8>,
        version: Option<u64>,
    ) -> Result<CommitmentProof, Self::Error>;

    /// Accept a batch ops (an op is either a DB insertion or a deletion), keep
    /// them in the memory, but do not persist to disk yet; also, increment the
    /// version.
    ///
    /// This is typically invoked in the ABCI `FinalizeBlock` call.
    fn flush_but_not_commit(&self, batch: Batch) -> Result<(u64, Option<Hash256>), Self::Error>;

    /// Persist pending data added in the `flush` method to disk.
    ///
    /// This is typically invoked in the ABCI `Commit` call.
    fn commit(&self) -> Result<(), Self::Error>;

    /// Flush and commit in one go.
    ///
    /// This is typically only invoked in the ABCI `InitChain` call.
    fn flush_and_commit(&self, batch: Batch) -> Result<(u64, Option<Hash256>), Self::Error> {
        let (new_version, root_hash) = self.flush_but_not_commit(batch)?;
        self.commit()?;
        Ok((new_version, root_hash))
    }
}

/// Represents a database that can be pruned.
///
/// These methods aren't used by the app, so we split them off into a separate
/// trait.
pub trait PrunableDb: Db {
    /// Return the oldest version available in the database.
    /// Versions older than this have been pruned.
    /// Return `None` if the DB hasn't been pruned once.
    fn oldest_version(&self) -> Option<u64>;

    /// Prune data of less or equal to the given version.
    ///
    /// That is, `up_to_version` will be thd oldest version available in the
    /// database post pruning.
    fn prune(&self, up_to_version: u64) -> Result<(), Self::Error>;
}

// ------------------------------------ vm -------------------------------------

/// Represents a virtual machine that can execute programs.
pub trait Vm: Sized {
    type Error: ToString;
    type Instance: Instance<Error = Self::Error>;

    /// Create an instance of the VM given a storage, a querier, and a guest
    /// program.
    ///
    /// Need a mutable reference (`&mut self`) because the VM might uses some
    /// sort of caching to speed up instance building.
    fn build_instance(
        &mut self,
        code: &[u8],
        code_hash: Hash256,
        storage: StorageProvider,
        state_mutable: bool,
        querier: QuerierProvider<Self>,
        query_depth: usize,
        gas_tracker: GasTracker,
    ) -> Result<Self::Instance, Self::Error>;
}

pub trait Instance {
    type Error: ToString;

    /// Call a function that takes exactly 0 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_0_out_1(self, name: &'static str, ctx: &Context) -> Result<Vec<u8>, Self::Error>;

    /// Call a function that takes exactly 1 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_1_out_1<P>(
        self,
        name: &'static str,
        ctx: &Context,
        param: &P,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P: AsRef<[u8]>;

    /// Call a function that takes exactly 2 input parameters (other than the
    /// context) and returns exactly 1 output.
    fn call_in_2_out_1<P1, P2>(
        self,
        name: &'static str,
        ctx: &Context,
        param1: &P1,
        param2: &P2,
    ) -> Result<Vec<u8>, Self::Error>
    where
        P1: AsRef<[u8]>,
        P2: AsRef<[u8]>;
}

// ----------------------------- proposal preparer -----------------------------

/// Represents a worker that processes the ABCI++ `PrepareProposal` request.
pub trait ProposalPreparer {
    type Error: ToString;

    /// Process the ABCI++ `PrepareProposal` request.
    ///
    /// The preparer is provided with a querier so that it can do its work based
    /// on the state of the chain.
    fn prepare_proposal(
        &self,
        querier: QuerierWrapper,
        txs: Vec<Bytes>,
        max_tx_bytes: usize,
    ) -> Result<Vec<Bytes>, Self::Error>;
}
