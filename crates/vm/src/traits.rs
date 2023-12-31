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

    /// The minimum bound is inclusive, maximum bound is exclusive. Return the
    /// iterator_id.
    ///
    /// If min > max, the iterator should just be an empty iterator. Don't error
    /// or panic in this case.
    //
    // note: the id has to be u32, not usize, because we need to pass it over
    // the wasm32 FFI.
    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32>;

    /// NOTE: If the iterator reaches end, it should be dropped to save memory.
    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<(Vec<u8>, Vec<u8>)>>;
}
