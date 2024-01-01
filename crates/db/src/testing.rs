use {
    crate::{Batch, Committable, Op, Storage},
    cw_std::{Order, Record},
    std::{collections::BTreeMap, ops::Bound},
};

// not to be confused with cw_std::MockStorage
#[derive(Default, Debug, Clone)]
pub struct MockStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.data
    }
}

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn range_next(
        &self,
        min:   Bound<Vec<u8>>,
        max:   Bound<Vec<u8>>,
        order: Order,
    ) -> anyhow::Result<Option<Record>> {
        Ok(btreemap_range_next(&self.data, min, max, order))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.data.remove(key);
        Ok(())
    }
}

impl Committable for MockStorage {
    fn apply(&mut self, batch: Batch) -> anyhow::Result<()> {
        for (key, op) in batch {
            if let Op::Put(value) = op {
                self.write(&key, &value)?;
            } else {
                self.remove(&key)?;
            }
        }
        Ok(())
    }
}

pub fn btreemap_range_next<K: Clone + Ord, V: Clone>(
    map:   &BTreeMap<K, V>,
    min:   Bound<K>,
    max:   Bound<K>,
    order: Order,
) -> Option<(K, V)> {
    let mut range = map.range((min, max));
    let record = match order {
        Order::Ascending => range.next(),
        Order::Descending => range.next_back(),
    };
    record.map(|(k, v)| (k.clone(), v.clone()))
}
