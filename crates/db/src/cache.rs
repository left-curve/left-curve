use {
    crate::{btreemap_range_next, Batch, Committable, Op, Storage},
    cw_std::{Order, Record},
    std::{cmp::Ordering, ops::Bound},
};

/// Adapted from cw-multi-test:
/// https://github.com/CosmWasm/cw-multi-test/blob/v0.19.0/src/transactions.rs#L170-L253
pub struct Cached<S> {
    base:    S,
    pending: Batch,
}

impl<S> Cached<S> {
    /// Create a new cached store with an empty write batch.
    pub fn new(base: S) -> Self {
        Self {
            base,
            pending: Batch::new(),
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
        self.base.apply(self.pending)?;
        Ok(self.base)
    }
}

impl<S> Committable for Cached<S>
where
    S: Storage,
{
    fn apply(&mut self, batch: Batch) -> anyhow::Result<()> {
        // this merges the two batches, with the incoming batch taking precedence.
        self.pending.extend(batch);
        Ok(())
    }
}

impl<S> Cached<S>
where
    S: Storage,
{
    fn take_pending(
        &self,
        pending_key: Vec<u8>,
        pending_op:  Op,
        min:         Bound<Vec<u8>>,
        max:         Bound<Vec<u8>>,
        order:       Order,
    ) -> anyhow::Result<Option<Record>> {
        if let Op::Put(value) = pending_op {
            return Ok(Some((pending_key, value)));
        }

        match order {
            Order::Ascending => self.range_next(Bound::Excluded(pending_key), max, order),
            Order::Descending => self.range_next(min, Bound::Excluded(pending_key), order),
        }
    }
}

impl<S> Storage for Cached<S>
where
    S: Storage,
{
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        match self.pending.get(key) {
            Some(Op::Put(value)) => Ok(Some(value.clone())),
            Some(Op::Delete) => Ok(None),
            None => self.base.read(key),
        }
    }

    fn range_next(
        &self,
        min:   Bound<Vec<u8>>,
        max:   Bound<Vec<u8>>,
        order: Order,
    ) -> anyhow::Result<Option<Record>> {
        let base_peek = self.base.range_next(min.clone(), max.clone(), order)?;
        let pending_peek = btreemap_range_next(&self.pending, min.clone(), max.clone(), order);

        match (base_peek, pending_peek) {
            (Some((base_key, base_value)), Some((pending_key, pending_op))) => {
                match (base_key.cmp(&pending_key), order) {
                    (Ordering::Less, Order::Ascending) | (Ordering::Greater, Order::Descending) => {
                        Ok(Some((base_key, base_value)))
                    },
                    _ => self.take_pending(pending_key, pending_op, min, max, order),
                }
            },
            (None, Some((pending_key, pending_op))) => {
                self.take_pending(pending_key, pending_op, min, max, order)
            },
            (Some(base_record), None) => Ok(Some(base_record)),
            (None, None) => Ok(None),
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.pending.insert(key.to_vec(), Op::Put(value.to_vec()));
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.pending.insert(key.to_vec(), Op::Delete);
        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, crate::MockStorage};

    // illustration of this test case:
    //
    // base    : 1 2 _ 4 5 6 7 _
    // pending :   D P _ _ P D 8  (P = put, D = delete)
    // merged  : 1 _ 3 4 5 6 _ 8
    fn make_test_case() -> anyhow::Result<(Cached<MockStorage>, Vec<Record>)> {
        let mut base = MockStorage::new();
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

        let merged = vec![
            (vec![1], vec![1]),
            (vec![3], vec![3]),
            (vec![4], vec![4]),
            (vec![5], vec![5]),
            (vec![6], vec![255]),
            (vec![8], vec![8]),
        ];

        Ok((cached, merged))
    }

    #[test]
    fn iterator_works() -> anyhow::Result<()> {
        let (cached, mut merged) = make_test_case()?;
        assert_eq!(cached.to_vec(Order::Ascending)?, merged);

        merged.reverse();
        assert_eq!(cached.to_vec(Order::Descending)?, merged);

        Ok(())
    }

    #[test]
    fn commit_works() -> anyhow::Result<()> {
        let (cached, merged) = make_test_case()?;

        let base = cached.commit()?;
        assert_eq!(base.to_vec(Order::Ascending)?, merged);

        Ok(())
    }

    // TODO: add fuzz test
}
