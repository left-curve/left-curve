use {
    crate::{HostState, Peekable},
    anyhow::anyhow,
    cw_std::{MockStorage, Order, Record, Storage},
    std::{collections::HashMap, iter, vec},
};

// not to be confused with cw_std::MockStorage
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct MockHostState {
    store:        MockStorage,
    iterators:    HashMap<u32, iter::Peekable<vec::IntoIter<Record>>>,
    next_iter_id: u32,
}

impl MockHostState {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_iterator_mut(
        &mut self,
        id: u32,
    ) -> anyhow::Result<&mut iter::Peekable<vec::IntoIter<Record>>> {
        self.iterators
            .get_mut(&id)
            .ok_or_else(|| anyhow!("[MockHostState]: can't find iterator with id {id}"))
    }
}

impl HostState for MockHostState {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.store.read(key))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.store.write(key, value);

        // delete all existing iterators to avoid race conditions
        // for more details on why do this, see the comments in HostState trait
        //
        // HashMap::clear deletes all entries but keeps the allocated memory.
        // this is probably more performant than making a new HashMap in most cases
        self.iterators.clear();

        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.store.remove(key);
        // delete all existing iterators, similar rationale as in `write`
        self.iterators.clear();
        Ok(())
    }

    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32> {
        let iterator_id = self.next_iter_id;
        // don't think we need to handle overflowing here. no way someone
        // creates u32::MAX as many iterators...?
        self.next_iter_id += 1;

        // for this mock, we clone all keys into memory
        // for production, we need to think of a more efficient approach
        //
        // no need to handle the case of min > max, because self.store.scan
        // already takes care of it
        let vec = self.store.scan(min, max, order).collect::<Vec<_>>();
        self.iterators.insert(iterator_id, vec.into_iter().peekable());

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<Record>> {
        self.get_iterator_mut(iterator_id).map(|iter| iter.next())
    }
}

impl Peekable for MockHostState {
    fn peek(&mut self, iterator_id: u32) -> anyhow::Result<Option<Record>> {
        self.get_iterator_mut(iterator_id).map(|iter| iter.peek().cloned())
    }
}
