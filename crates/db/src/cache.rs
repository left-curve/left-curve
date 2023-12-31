use {
    crate::{Batch, Committable, Op},
    cw_std::{Order, Record},
    cw_vm::{HostState, Peekable},
    std::{
        cmp::Ordering,
        collections::{BTreeMap, HashMap},
        ops::Bound,
    },
};

/// Adapted from cw-multi-test:
/// https://github.com/CosmWasm/cw-multi-test/blob/v0.19.0/src/transactions.rs#L170-L253
pub struct Cached<S> {
    base:         S,
    pending:      Batch,
    iterators:    HashMap<u32, CachedIter>,
    next_iter_id: u32,
}

impl<S> Cached<S> {
    /// Create a new cached store with an empty write batch.
    pub fn new(base: S) -> Self {
        Self {
            base,
            pending:      Batch::new(),
            iterators:    HashMap::new(),
            next_iter_id: 0,
        }
    }

    /// Comsume self, discard the uncommitted batch, return the underlying store.
    pub fn recycle(self) -> S {
        self.base
    }
}

impl<S> Cached<S>
where
    S: Committable,
{
    /// Consume the cached store, write all ops to the underlying store, return
    /// the underlying store.
    pub fn commit(mut self) -> anyhow::Result<S> {
        self.base.commit(self.pending)?;
        Ok(self.base)
    }
}

impl<S> HostState for Cached<S>
where
    S: HostState + Peekable,
{
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        match self.pending.get(key) {
            Some(Op::Put(value)) => Ok(Some(value.clone())),
            Some(Op::Delete) => Ok(None),
            None => self.base.read(key),
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.pending.insert(key.to_vec(), Op::Put(value.to_vec()));

        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions
        self.iterators.clear();

        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.pending.insert(key.to_vec(), Op::Delete);

        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions
        self.iterators.clear();

        Ok(())
    }

    fn scan(
        &mut self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<u32> {
        let iterator_id = self.next_iter_id;
        self.next_iter_id += 1;

        let base_iter_id = self.base.scan(min, max, order)?;
        let iterator = CachedIter::new(base_iter_id, min, max, order);
        self.iterators.insert(iterator_id, iterator);

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: u32) -> anyhow::Result<Option<Record>> {
        // let (iter, pending) = self.get_iter_and_batch_mut(iterator_id)?;
        // iter.next(pending)
        let iter = self.iterators.get_mut(&iterator_id).unwrap();
        iter.next(&mut self.base, &self.pending)
    }
}

struct CachedIter {
    base_iter_id: u32,
    pending_curr: Option<Vec<u8>>,
    pending_end: Option<Vec<u8>>,
    order: Order,
}

impl CachedIter {
    pub fn new(base_iter_id: u32, min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> Self {
        let (pending_curr, pending_end) = match order {
            Order::Ascending => (min, max),
            Order::Descending => (max, min),
        };

        Self {
            base_iter_id,
            pending_curr: pending_curr.map(|bytes| bytes.to_vec()),
            pending_end: pending_end.map(|bytes| bytes.to_vec()),
            order,
        }
    }

    // we can't implement the actual Iterator trait, because CachedIter needs to
    // hold a reference of the Batch, with which we run into lifetime issues...
    //
    // in this implementation, each `next` call involves walking the BTree, which
    // isn't optimal, but I don't have a better idea for now.
    //
    // perhaps later we can do something with unsafe Rust...
    pub fn next<S>(&mut self, base: &mut S, pending: &Batch) -> anyhow::Result<Option<Record>>
    where
        S: HostState + Peekable,
    {
        let pending_peek = btreemap_next_key(
            pending,
            self.pending_curr.as_ref(),
            self.pending_end.as_ref(),
            self.order,
        );
        let base_peek = base.peek(self.base_iter_id)?;

        match (base_peek, pending_peek) {
            // neither base and pending has reached end
            (Some((base_key, _)), Some((pending_key, pending_op))) => {
                let mut order = base_key.cmp(pending_key);
                if self.order == Order::Descending {
                    order = order.reverse();
                }

                match order {
                    Ordering::Less => base.next(self.base_iter_id),
                    Ordering::Equal => {
                        base.next(self.base_iter_id)?;
                        self.take_pending(base, pending, pending_key, pending_op)
                    },
                    Ordering::Greater => self.take_pending(base, pending, pending_key, pending_op),
                }
            },

            // base has reached end, pending not --> advance pending
            // but if pending is a Delete, then skip it
            (None, Some((key, op))) => self.take_pending(base, pending, key, op),

            // pending has reached end, base has not --> advance base
            (Some(_), None) => base.next(self.base_iter_id),

            // both have reached end --> simply return None
            (None, None) => Ok(None),
        }
    }

    fn take_pending<S>(
        &mut self,
        base: &mut S,
        pending: &Batch,
        key: &[u8],
        op: &Op,
    ) -> anyhow::Result<Option<Record>>
    where
        S: HostState + Peekable,
    {
        self.pending_curr = Some(key.to_vec());
        match op {
            Op::Put(value) => Ok(Some((key.to_vec(), value.to_vec()))),
            Op::Delete => self.next(base, pending),
        }
    }
}

fn btreemap_next_key<'a, K: Ord, V>(
    map: &'a BTreeMap<K, V>,
    curr: Option<&K>,
    end: Option<&K>,
    order: Order,
) -> Option<(&'a K, &'a V)> {
    let bounds = match order {
        Order::Ascending => (
            curr.map_or(Bound::Unbounded, Bound::Excluded),
            end.map_or(Bound::Unbounded, Bound::Excluded),
        ),
        Order::Descending => (
            end.map_or(Bound::Unbounded, Bound::Included),
            curr.map_or(Bound::Unbounded, Bound::Excluded),
        ),
    };

    match order {
        Order::Ascending => map.range(bounds).next(),
        Order::Descending => map.range(bounds).next_back(),
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, cw_vm::MockHostState};

    #[test]
    fn cached_iterator_works() -> anyhow::Result<()> {
        let mut base = MockHostState::new();
        base.write(&[1], &[1])?;
        base.write(&[2], &[2])?;
        base.write(&[4], &[4])?;
        base.write(&[5], &[5])?;
        base.write(&[6], &[6])?;
        base.write(&[7], &[7])?;


        let mut cached = Cached::new(base);
        cached.remove(&[2])?;
        cached.write(&[3], &[3])?;
        cached.write(&[6], &[255])?;
        cached.remove(&[7])?;
        cached.write(&[8], &[8])?;

        let iterator_id = cached.scan(None, None, Order::Ascending)?;
        assert_eq!(cached.next(iterator_id)?, Some((vec![1], vec![1])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![5], vec![5])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![6], vec![255])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![8], vec![8])));
        assert_eq!(cached.next(iterator_id)?, None);

        let iterator_id = cached.scan(None, None, Order::Descending)?;
        assert_eq!(cached.next(iterator_id)?, Some((vec![8], vec![8])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![6], vec![255])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![5], vec![5])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(cached.next(iterator_id)?, Some((vec![1], vec![1])));
        assert_eq!(cached.next(iterator_id)?, None);

        Ok(())
    }

    // TODO: add fuzz test
}
