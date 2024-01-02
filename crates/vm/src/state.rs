use {
    anyhow::anyhow,
    cw_std::{Order, Record, Storage},
    std::{collections::HashMap, mem},
};

// wraps a cw_vm::Storage, and implements the necessary Wasm import functions.
pub struct HostState<S> {
    store:        S,
    iterators:    HashMap<u32, Box<dyn Iterator<Item = Record>>>,
    next_iter_id: u32,
}

impl<S> HostState<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            iterators:    HashMap::new(),
            next_iter_id: 0,
        }
    }

    /// Consume self, return the owned instance of the storage.
    pub fn disassemble(self) -> S {
        self.store
    }
}

impl<S> HostState<S>
where
    S: Storage,
{
    pub fn db_read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        self.store.read(key)
    }

    // create a new iterator with the given bounds (min is inclusive, max is
    // exclusive) and iteration order. return a unique iterator ID.
    //
    // IMPORTANT NOTE: whereas this method takes a `&mut self`, it must NOT
    // mutate the underlying KV store data!
    pub fn db_scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32> {
        // need to handle overflow here? no one will realistically create
        // u32::MAX as many iterators without exceeding the gas limit...
        let iterator_id = self.next_iter_id;
        self.next_iter_id += 1;

        // use unsafe rust to ignore lifetime check here. this is really tricky...
        // let me try explain:
        //
        // the iter above, has &'1 lifetime, where '1 is the lifetime of the
        // &mut self in the function signature.
        //
        // this means, this iterator goes out of scope right at the end of this
        // function. this doesn't work with our use case. we need to save the
        // iterator, can call `next` on it later.
        //
        // therefore, we use std::mem::transmute here to turn the '1 iterator
        // into a 'static iterator (thanks ChatGPT for suggesting this.)
        //
        // let's think about if this is safe. the lifetime here safeguards two
        // things:
        //
        // 1. the iterator references data in the Storage instance that this
        //    HostState holds. therefore the iterator must not live longer than
        //    the HostState.
        //
        //    this is perfectly fine. the HostState is dropped at the end of the
        //    wasm execution, with all iterators dropped at the same time.
        //
        // 2. the iterator holds an immutable reference to the Storage instance.
        //    this means until the iterator is dropped, data in the Storage can't
        //    be mutated.
        //
        //    we make sure of this in db_{write,remove} function. whenever data
        //    is mutated, we delete all existing iterators.
        //
        // so overall this should be safe.
        let iter = self.store.scan(min, max, order)?;
        let iter_static = unsafe { mem::transmute(iter) };
        self.iterators.insert(iterator_id, iter_static);

        Ok(iterator_id)
    }

    // IMPORTANT NOTE: whereas this method takes a `&mut self`, it must NOT
    // mutate the underlying KV store data!
    pub fn db_next(&mut self, iterator_id: u32) -> anyhow::Result<Option<Record>> {
        self.iterators
            .get_mut(&iterator_id)
            .ok_or_else(|| anyhow!("[HostState]: iterator not found with id `{iterator_id}`"))
            .map(|iter| iter.next())
    }

    pub fn db_write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.store.write(key, value)?;

        // IMPORTANT: delete all existing iterators whenever KV data is mutated.
        // see comments in db_scan for rationale.
        self.iterators.clear();

        Ok(())
    }

    pub fn db_remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.store.remove(key)?;

        // IMPORTANT: delete all existing iterators whenever KV data is mutated.
        self.iterators.clear();

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, cw_std::{MockStorage, Order}};

    #[test]
    fn host_state_iterator_works() -> anyhow::Result<()> {
        let mut hs = HostState::new(MockStorage::new());
        hs.db_write(&[1], &[1])?;
        hs.db_write(&[2], &[2])?;
        hs.db_write(&[3], &[3])?;
        hs.db_write(&[4], &[4])?;
        hs.db_write(&[5], &[5])?;

        // iterate ascendingly. note that min bound is inclusive
        let iterator_id = hs.db_scan(Some(&[2]), None, Order::Ascending)?;
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![2], vec![2])));
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![5], vec![5])));
        assert_eq!(hs.db_next(iterator_id)?, None);

        // iterate descendingly. note that max bound is exclusive
        let iterator_id = hs.db_scan(Some(&[3]), Some(&[5]), Order::Descending)?;
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(hs.db_next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(hs.db_next(iterator_id)?, None);

        // calling db_next again after the iterator has reached end should just
        // return None, without error
        assert_eq!(hs.db_next(iterator_id)?, None);

        Ok(())
    }
}
