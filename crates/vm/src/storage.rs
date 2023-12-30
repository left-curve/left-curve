use {
    anyhow::{anyhow, bail},
    cw_sdk::{MockStorage, Order, Storage},
    std::{collections::HashMap, vec},
};

// not to be confused with cw_sdk::Storage.
//
// compared with cw_sdk::Storage, this trait has the following differences:
// - the methods are fallible
// - iteration methods. the scan methods uses a mutable reference, returns an
//   iterator_id instead of the actual iterator. use the next method to advance
//   the iterator
// - responses include gas consumption info (TODO)
pub trait BackendStorage {
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

// not to be confused with cw_sdk::MockStorage
#[derive(Default)]
pub struct MockBackendStorage {
    inner:        MockStorage,
    iterators:    HashMap<u32, vec::IntoIter<(Vec<u8>, Vec<u8>)>>,
    next_iter_id: u32,
}

impl MockBackendStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendStorage for MockBackendStorage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.inner.read(key))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.inner.write(key, value);
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.inner.remove(key);
        Ok(())
    }

    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32> {
        let iterator_id = self.next_iter_id;
        self.next_iter_id = iterator_id.checked_add(1).ok_or(anyhow!("Too many iterators"))?;

        // for this mock, we clone all keys into memory
        // for production, we need to think of a more efficient approach
        let vec = self.inner.scan(min, max, order).collect::<Vec<_>>();
        self.iterators.insert(iterator_id, vec.into_iter());

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<(Vec<u8>, Vec<u8>)>> {
        let Some(iter) = self.iterators.get_mut(&iterator_id) else {
            bail!("Can't find iterator with id {iterator_id}");
        };

        Ok(iter.next())
    }
}
