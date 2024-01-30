use {
    crate::{VmError, VmResult},
    cw_std::{GenericResult, Order, QueryRequest, QueryResponse, Record},
};

/// Describing an object that can perform queries.
pub trait BackendQuerier {
    fn query_chain(&self, req: QueryRequest) -> VmResult<GenericResult<QueryResponse>>;
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
/// - There isn't a method equivalent to `flush` because we don't need it for
///   out use case.
pub trait BackendStorage {
    type Err: Into<VmError>;

    fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Err>;

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
    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Result<i32, Self::Err>;

    /// Advance the iterator with the given ID.
    ///
    /// IMPORTANT: Same as `scan`, despite we are given a `&mut self`,
    /// we MUST NOT change the underlying KV data.
    fn next(&mut self, iterator_id: i32) -> Result<Option<Record>, Self::Err>;

    /// IMPORTANT: to avoid race conditions, calling this method MUST result in
    /// all existing iterators being dropped.
    fn write(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Err>;

    /// IMPORTANT: to avoid race conditions, calling this method MUST result in
    /// all existing iterators being dropped.
    fn remove(&mut self, key: &[u8]) -> Result<(), Self::Err>;
}
