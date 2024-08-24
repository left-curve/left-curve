use grug::{Binary, ByteArray, Empty, Hash256, Hash512};

pub type InstantiateMsg = Empty;

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Perform an infinite loop. Test if the VM can properly halt execution
    /// when gas is exhausted.
    InfiniteLoop {},
    /// See `QueryMsg::ForceWrite`.
    ///
    /// This tests that the VM can not only reject a forced write when calling
    /// the `query` export, but also when handling the `query_chain` import call
    /// within an `execute` call.
    ForceWriteOnQuery { key: String, value: String },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Run a loop of the given number of iterations. Within each iteration, a
    /// set of math operations (addition, subtraction, multiplication, division)
    /// are performed.
    ///
    /// This is used for deducing the relation between Wasmer gas metering
    /// points and CPU time (i.e. how many gas points roughly correspond to one
    /// second of run time).
    #[returns(())]
    Loop { iterations: u64 },
    /// Attempt to write a key-value pair to the contract storage.
    ///
    /// If using the Grug library, this is impossible to do, because the
    /// contract is given an `ImmutableCtx` in the `query` function, which
    /// doesn't come with state mutable methods.
    ///
    /// However, a malicious contract can attempt to directly call the `db_write`,
    /// `db_remove`, or `db_remove_range` FFI import methods directly. We need
    /// to test whether the VM can properly reject this behavior.
    #[returns(())]
    ForceWrite { key: String, value: String },
    /// Verify a single Secp256r1 signature.
    #[returns(())]
    VerifySecp256r1 {
        pk: Binary,
        sig: ByteArray<64>,
        msg_hash: Hash256,
    },
    /// Verify a single Secp256k1 signature.
    #[returns(())]
    VerifySecp256k1 {
        pk: Binary,
        sig: ByteArray<64>,
        msg_hash: Hash256,
    },
    /// Recover an Secp256k1 publick key from a signature.
    #[returns(Binary)]
    RecoverSepc256k1 {
        sig: ByteArray<64>,
        msg_hash: Hash256,
        recovery_id: u8,
        compressed: bool,
    },
    /// Verify a single Ed25519 signature.
    #[returns(())]
    VerifyEd25519 {
        pk: ByteArray<32>,
        sig: ByteArray<64>,
        msg_hash: Hash512,
    },
    /// Verify a batch of Ed25519 signatures.
    #[returns(())]
    VerifyEd25519Batch {
        pks: Vec<ByteArray<32>>,
        sigs: Vec<ByteArray<64>>,
        prehash_msgs: Vec<Binary>,
    },
}
