use cw_std::Order;

// not to be confused with cw_std::Storage.
//
// compared with cw_std::Storage, this trait has the following differences:
// - the methods are fallible
// - iteration methods. the scan methods uses a mutable reference, returns an
//   iterator_id instead of the actual iterator. use the next method to advance
//   the iterator
// - responses include gas consumption info (TODO)
pub trait HostState {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()>;

    /// Create an iterator over the KV data. Return a unique iterator ID.
    ///
    /// The minimum bound is inclusive, maximum is exclusive. If min > max, just
    /// create an empty iterator. Don't error or panic in this case.
    ///
    /// IMPORTANT implementation considerations!!!
    /// - Calling `scan` must NOT mutate the underlying KV data. However we may
    ///   need to mutate the HostState state, so this requires a mutable reference.
    /// - Calling `write` or `remove` should result in all existing iterators
    ///   being dropped.
    ///   Consider this situation: we have the following keys [1, 2, 4, 5]. An
    ///   iterator is currently at 2 (ascending order). What if we insert a new
    ///   key "3" and then call `next`, should the iterator return 3 or 4? This
    ///   is a race condition.
    ///   In pure Rust, this would be a compile time error, because the iterator
    ///   holds an immutable reference to the KV store, so no mutation to it can
    ///   be made until the iterator is dropped. However we're working with the
    ///   Rust<>Wasm FFI here, so rustc can't help us. The HostState implementation
    ///   must include a mechanism to avoid this race conditon.
    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32>;

    /// Similar to `scan`, whereas this function takes a `&mut self`, it must
    /// NOT mutate the underlying KV data.
    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<(Vec<u8>, Vec<u8>)>>;
}
