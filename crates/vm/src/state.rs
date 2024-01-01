use {
    anyhow::anyhow,
    cw_db::Storage,
    cw_std::{Order, Record},
    std::{collections::HashMap, ops::Bound},
};

// wraps a cw_vm::Storage, and implements the necessary Wasm import functions.
pub struct HostState<S> {
    store:        S,
    iterators:    HashMap<u32, HostStateIter>,
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
        let iterator_id = self.next_iter_id;
        // need to handle overflow here? no one will realistically create
        // u32::MAX as many iterators without exceeding the gas limit...
        self.next_iter_id += 1;

        let iterator = HostStateIter::new(min, max, order);
        self.iterators.insert(iterator_id, iterator);

        Ok(iterator_id)
    }

    // IMPORTANT NOTE: whereas this method takes a `&mut self`, it must NOT
    // mutate the underlying KV store data!
    pub fn db_next(&mut self, iterator_id: u32) -> anyhow::Result<Option<Record>> {
        self.iterators
            .get_mut(&iterator_id)
            .ok_or_else(|| anyhow!("[HostState]: iterator not found with id `{iterator_id}`"))?
            .next(&self.store)
    }

    pub fn db_write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.store.write(key, value)?;

        // IMPORTANT NOTE: to avoid race conditions, mutating the KV store data
        // results in all existing iterators being dropped!
        //
        // think of this this way: each iterator essentially holds an immutable
        // reference to the KV store (or a read lock if you like to think about
        // multithreading). before all iterators are dropped (or locks unlocked),
        // we should NOT be able to mutate the KV store. in pure Rust this would
        // be a compile time error. however we're working with Rust<>Wasm FFI so
        // the compiler can't help us here. we need to implement mechanisms
        // ourselves to prevent this race condition.
        self.iterators.clear();

        Ok(())
    }

    pub fn db_remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.store.remove(key)?;

        // delete all existing iterators, same rationale as in `write`
        self.iterators.clear();

        Ok(())
    }
}

struct HostStateIter {
    min:   Bound<Vec<u8>>,
    max:   Bound<Vec<u8>>,
    order: Order,
    ended: bool,
}

impl HostStateIter {
    pub fn new(min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> Self {
        // min is inclusive, max is exclusive
        Self {
            min: min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec())),
            max: max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec())),
            order,
            ended: false,
        }
    }

    pub fn next<S: Storage>(&mut self, store: &S) -> anyhow::Result<Option<Record>> {
        if self.ended {
            return Ok(None);
        }

        // TODO: can avoid cloning one of min & max here, depending on the order
        let Some((k, v)) = store.range_next(self.min.clone(), self.max.clone(), self.order)? else {
            self.ended = true;
            return Ok(None);
        };

        match self.order {
            Order::Ascending => self.min = Bound::Excluded(k.clone()),
            Order::Descending => self.max = Bound::Excluded(k.clone()),
        }

        Ok(Some((k, v)))
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, cw_db::MockStorage, cw_std::Order};

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
