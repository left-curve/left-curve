//! This file describes the import API that the host provides to Wasm modules.
//!
//! Three types of import functions are provided:
//!
//! - database reads/writes,
//! - cryptography methods, and
//! - a method for querying the chain.
//!
//! These functions are abstracted into the `Storage`, `Api`, and `Querier`
//! traits.

use {
    crate::{Addr, Batch, Op, Order, QueryRequest, QueryResponse, Record, StdResult},
    dyn_clone::DynClone,
};

// ---------------------------------- storage ----------------------------------

/// Describing a KV store that supports read, write, and iteration.
///
/// Note that the store must be clone-able, which is required by Wasmer runtime.
/// We can't use the std library Clone trait, which is not object-safe.
/// We use [DynClone](https://crates.io/crates/dyn-clone) instead, which is
/// object-safe, and use the `clone_trait_object!` macro below to derive std
/// Clone trait for any type that implements Storage.
///
/// The object must also be Send and Sync, which is required by Wasmer runtime.
pub trait Storage: DynClone + Send + Sync {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If min > max, an empty iterator is to be returned.
    ///
    /// NOTE: Rust's BTreeMap panics if max > max. We don't want this behavior.
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    /// Perform a batch of writes and removes altogether, ideally atomically.
    ///
    /// The batch is provided by value instead of by reference (unlike other
    /// trait methods above) because in some implementations a copy/clone can be
    /// avoided this way, improving performance.
    ///
    /// The default implementation here is just looping through the ops and
    /// applying them one by one, which is inefficient and not atomic.
    /// Overwrite this implementation if there are more efficient approaches.
    fn flush(&mut self, batch: Batch) {
        for (key, op) in batch {
            if let Op::Insert(value) = op {
                self.write(&key, &value);
            } else {
                self.remove(&key);
            }
        }
    }
}

// a boxed storage is also a storage.
// this is necessary for use in `grug_app::execute::handle_submessage` (see the
// comment there for an explanation)
impl Storage for Box<dyn Storage> {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.as_ref().read(key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        self.as_ref().scan(min, max, order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.as_mut().write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.as_mut().remove(key)
    }

    fn flush(&mut self, batch: Batch) {
        self.as_mut().flush(batch)
    }
}

// derive std Clone trait for any type that implements Storage
dyn_clone::clone_trait_object!(Storage);

// ------------------------------------ api ------------------------------------

// note: I prefer to use generics (e.g. `impl AsRef<[u8]>`) instead of `&[u8]`
// for the method parameters, but by doing that the trait won't be object-safe
// (i.e. we won't be able to do `&dyn Api`). traits with methods that have
// generic methods can't be object-safe.
pub trait Api {
    /// Send a message to the host, which will be printed to the host's logging.
    /// Takes two arguments: the contract's address as raw bytes, and the message
    /// as UTF-8 bytes.
    ///
    /// Note: unlike Rust's built-in `dbg!` macro, which is only included in
    /// debug builds, this `debug` method is also included in release builds,
    /// and incurs gas cost. Make sure to comment this out before compiling your
    /// contracts.
    fn debug(&self, addr: &Addr, msg: &str);

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

    /// Recover the compressed byte of the `public key` from the `signature` and `message hash`.
    /// - **r**: the first `32 bytes` of the signature;
    /// - **s**: the last `32 bytes` of the signature;
    /// - **v**: the `recovery id`.
    ///
    /// Note: this function takes the hash of the message, not the prehash.
    fn secp256k1_pubkey_recover(
        &self,
        msg_hash: &[u8],
        sig: &[u8],
        recovery_id: u8,
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
        msgs_hash: &[&[u8]],
        sigs: &[&[u8]],
        pks: &[&[u8]],
    ) -> StdResult<()>;
}

// ---------------------------------- querier ----------------------------------

pub trait Querier {
    /// Make a query. This is the only method that the context needs to manually
    /// implement. The other methods will be implemented automatically.
    fn query_chain(&self, req: QueryRequest) -> StdResult<QueryResponse>;
}
