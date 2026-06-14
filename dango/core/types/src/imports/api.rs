use crate::{Addr, StdResult};

// Note: I prefer to use generics (e.g. `impl AsRef<[u8]>`) over `&[u8]` for
// input data, but by doing that the trait won't be object-safe (i.e. we won't
// be able to do `&dyn Api`). Traits with methods that have generic parameters
// can't be object-safe.
//
// Also note that trait methods must include `&self` in order to be object-safe.
pub trait Api {
    /// Send a message to the host, which will be printed to the host's logging.
    /// Takes two arguments: the contract's address as raw bytes, and the message
    /// as UTF-8 bytes.
    ///
    /// Note: unlike Rust's built-in `dbg!` macro, which is only included in
    /// debug builds, this `debug` method is also included in release builds,
    /// and incurs gas cost. Make sure to comment this out before compiling your
    /// contracts.
    fn debug(&self, addr: Addr, msg: &str);

    /// Verify an Secp256r1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()>;

    /// Verify an Secp256k1 signature with the given hashed message and public
    /// key.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()>;

    /// Recover the Secp256k1 public key from the signature over a message.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256k1_pubkey_recover(
        &self,
        msg_hash: &[u8],
        sig: &[u8],
        recovery_id: u8,
        compressed: bool,
    ) -> StdResult<Vec<u8>>;

    /// Verify an ED25519 signature with the given hashed message and public
    /// key.
    ///
    /// NOTE: This function takes the hash of the message, not the prehash.
    fn ed25519_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()>;

    /// Verify a batch of ED25519 signatures with the given hashed message and public
    /// key.
    /// NOTE: This function takes the hash of the messages, not the prehash.
    fn ed25519_batch_verify(
        &self,
        prehash_msgs: &[&[u8]],
        sigs: &[&[u8]],
        pks: &[&[u8]],
    ) -> StdResult<()>;

    /// Perform the SHA2-256 hash.
    fn sha2_256(&self, data: &[u8]) -> [u8; 32];

    /// Perform the SHA2-512 hash.
    fn sha2_512(&self, data: &[u8]) -> [u8; 64];

    /// Perform the SHA2-512 hash, truncated to the first 32 bytes.
    fn sha2_512_truncated(&self, data: &[u8]) -> [u8; 32];

    /// Perform the SHA3-256 hash.
    fn sha3_256(&self, data: &[u8]) -> [u8; 32];

    /// Perform the SHA3-512 hash.
    fn sha3_512(&self, data: &[u8]) -> [u8; 64];

    /// Perform the SHA3-512 hash, truncated to the first 32 bytes.
    fn sha3_512_truncated(&self, data: &[u8]) -> [u8; 32];

    /// Perform the Keccak-256 hash.
    fn keccak256(&self, data: &[u8]) -> [u8; 32];

    /// Perform the BLAKE2s-256 hash.
    fn blake2s_256(&self, data: &[u8]) -> [u8; 32];

    /// Perform the BLAKE2b-512 hash.
    fn blake2b_512(&self, data: &[u8]) -> [u8; 64];

    /// Perform the BLAKE3 hash.
    fn blake3(&self, data: &[u8]) -> [u8; 32];
}
