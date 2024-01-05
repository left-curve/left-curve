use {
    crate::Storage,
    anyhow::anyhow,
    cw_std::{Order, Record},
    std::{
        collections::{BTreeMap, HashMap},
        iter::Peekable,
        ops::Bound,
        vec,
    },
};

#[derive(Default, Debug, Clone)]
pub struct MockStorage {
    data:         BTreeMap<Vec<u8>, Vec<u8>>,
    iterators:    HashMap<i32, MockIter>,
    next_iter_id: i32,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_iterator_mut(&mut self, iterator_id: i32) -> anyhow::Result<&mut MockIter> {
        self.iterators
            .get_mut(&iterator_id)
            .ok_or_else(|| anyhow!("[MockStorage]: can't find iterator with id {iterator_id}"))
    }
}

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<i32> {
        let iterator_id = self.next_iter_id;
        self.next_iter_id += 1;

        let iterator = MockIter::new(&self.data, min, max, order);
        self.iterators.insert(iterator_id, iterator);

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: i32) -> anyhow::Result<Option<Record>> {
        self.get_iterator_mut(iterator_id).map(|iterator| iterator.next())
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions.
        self.iterators.clear();
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.data.remove(key);
        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions.
        self.iterators.clear();
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct MockIter {
    records: Peekable<vec::IntoIter<Record>>,
}

impl MockIter {
    pub fn new(data: &BTreeMap<Vec<u8>, Vec<u8>>, min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> Self {
        // if min > max, just make an empty iterator
        // BTreeMap would panic in this case
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Self {
                    records: Vec::new().into_iter().peekable(),
                };
            }
        }

        let min = min.map_or(Bound::Unbounded, |min| Bound::Included(min.to_vec()));
        let max = max.map_or(Bound::Unbounded, |max| Bound::Excluded(max.to_vec()));

        // for this mock, we just clone all records in the range into the
        // iterator object. this is apparent memory inefficient and not something
        // we should do for production
        let mut records = data
            .range((min, max))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        if order == Order::Descending {
            records.reverse();
        }

        Self {
            records: records.into_iter().peekable(),
        }
    }
}

impl Iterator for MockIter {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        self.records.next()
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_state_iterator_works() -> anyhow::Result<()> {
        let mut store = MockStorage::new();
        store.write(&[1], &[1])?;
        store.write(&[2], &[2])?;
        store.write(&[3], &[3])?;
        store.write(&[4], &[4])?;
        store.write(&[5], &[5])?;

        // iterate ascendingly. note that min bound is inclusive
        let iterator_id = store.scan(Some(&[2]), None, Order::Ascending)?;
        assert_eq!(store.next(iterator_id)?, Some((vec![2], vec![2])));
        assert_eq!(store.next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(store.next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(store.next(iterator_id)?, Some((vec![5], vec![5])));
        assert_eq!(store.next(iterator_id)?, None);

        // iterate descendingly. note that max bound is exclusive
        let iterator_id = store.scan(Some(&[3]), Some(&[5]), Order::Descending)?;
        assert_eq!(store.next(iterator_id)?, Some((vec![4], vec![4])));
        assert_eq!(store.next(iterator_id)?, Some((vec![3], vec![3])));
        assert_eq!(store.next(iterator_id)?, None);

        // calling db_next again after the iterator has reached end should just
        // return None, without error
        assert_eq!(store.next(iterator_id)?, None);

        Ok(())
    }
}
