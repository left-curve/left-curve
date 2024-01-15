use crate::{Batch, DbResult, Order, Record};

/// Describing a KV store that supports read, write, and iteration.
///
/// A question you may have is why these methods are not fallible (that is, for
/// example, why `read` returns an Option<Vec<u8>> instead of a Result<Option<Vec<u8>>>).
/// Surely reading/writing a database may fail?
///
/// The answer is that this trait describe the KV store _viewed from the Wasm
/// module's perspective_. Indeed DB reads/writes may fail, but if they fail,
/// the contract call is aborted in the host function; the Wasm module never
/// receives a response. As long as the Wasm module receives a response, the
/// read/write must have been successful.
pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If min > max, an empty iterator is to be returned.
    ///
    /// NOTE: Rust's BTreeMap panics if max > max. We don't want this behavior.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);
}

/// Describing a KV store that supports read, write, and iteration.
///
/// This is related to `cw_std::Storage` trait, but do not confuse them. The std
/// trait is the KV store viewed from the Wasm module's perspective, while this
/// one is viewed form the host's perspective. There are two key distinctions:
/// - The read/write/remove methods here are fallible.
/// - The `scan` method, instead of returning an iterator object, just returns
///   an iterator ID. To advance the iterator, call the `next` method with the
///   ID. The reason for this is we can't pass an iterator object over the
///   Rust<>Wasm FFI; we can only pass IDs.
pub trait BackendStorage {
    fn read(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>>;

    /// Create an iterator with the given bounds and order. Return an integer
    /// identifier of the itereator created.
    ///
    /// Same as in `cw_std::Storage` trait, minimum bound is inclusive, while
    /// maximum bound is exclusive. If min > max, instead of panicking, simply
    /// create an empty iterator.
    ///
    /// IMPORTANT: This methods takes a `&mut self`, because typically we store
    /// the iterators in a HashMap inside the storage object, which needs to be
    /// updated. Despite given a mutable reference, this method MUST NOT change
    /// the underlying KV data.
    fn scan(&mut self, min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> DbResult<i32>;

    /// Advance the iterator with the given ID.
    ///
    /// IMPORTANT: Same as `scan`, despite we are given a `&mut self`,
    /// we MUST NOT change the underlying KV data.
    fn next(&mut self, iterator_id: i32) -> DbResult<Option<Record>>;

    fn write(&mut self, key: &[u8], value: &[u8]) -> DbResult<()>;

    fn remove(&mut self, key: &[u8]) -> DbResult<()>;
}

/// Describing a KV store capable to performing a batch of reads/writes together,
/// ideally atomically.
pub trait Flush {
    fn flush(&mut self, batch: Batch) -> DbResult<()>;
}
