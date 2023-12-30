use {
    crate::HostState,
    anyhow::{anyhow, bail},
    cw_sdk::{MockStorage, Order, Storage},
    std::{collections::HashMap, vec},
};

// not to be confused with cw_sdk::MockStorage
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct MockHostState {
    store:        MockStorage,
    iterators:    HashMap<u32, vec::IntoIter<(Vec<u8>, Vec<u8>)>>,
    next_iter_id: u32,
}

impl MockHostState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl HostState for MockHostState {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.store.read(key))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.store.write(key, value);
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.store.remove(key);
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
        let vec = self.store.scan(min, max, order).collect::<Vec<_>>();
        self.iterators.insert(iterator_id, vec.into_iter());

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<(Vec<u8>, Vec<u8>)>> {
        let Some(iter) = self.iterators.get_mut(&iterator_id) else {
            bail!("[MockHostState]: can't find iterator with id {iterator_id}");
        };

        Ok(iter.next())
    }
}
