use {
    grug_types::Op,
    std::{
        borrow::Borrow,
        collections::BTreeMap,
        marker::PhantomData,
        ops::{Bound, RangeBounds},
    },
};

pub struct VersionedMap<K, V> {
    // initialized to None
    // set to 0 the first time a batch is written
    // incremented by 1 each following batch write
    pub latest_version: Option<u64>,
    // key => (version => op)
    nested_map: BTreeMap<K, BTreeMap<u64, Op<V>>>,
}

impl<K, V> VersionedMap<K, V> {
    pub fn new() -> Self {
        Self {
            latest_version: None,
            nested_map: BTreeMap::new(),
        }
    }
}

impl<K, V> Default for VersionedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> VersionedMap<K, V>
where
    K: Ord,
{
    pub fn write_batch<B>(&mut self, batch: B)
    where
        B: IntoIterator<Item = (K, Op<V>)>,
    {
        let version = match self.latest_version.as_mut() {
            None => {
                self.latest_version = Some(0);
                0
            },
            Some(version) => {
                *version += 1;
                *version
            },
        };

        for (key, op) in batch {
            self.nested_map.entry(key).or_default().insert(version, op);
        }
    }

    pub fn get<T>(&self, key: &T, version: u64) -> Option<&V>
    where
        T: Ord + ?Sized,
        K: Borrow<T>,
    {
        let latest_version = self.latest_version?;
        if version > latest_version {
            panic!("version that is newer than the latest ({version} > {latest_version})");
        }
        self.nested_map
            .get(key)?
            .range(0..=version)
            .last()
            .and_then(|(_, op)| op.as_ref().into_option())
    }

    pub fn range<R, T: ?Sized>(
        &self,
        range: R,
        version: u64,
    ) -> VersionedIterator<'_, K, V, R, T>
    where
        K: Borrow<T>,
        T: Ord,
        R: RangeBounds<T>,
    {
        if let Some(latest_version) = self.latest_version {
            if version > latest_version {
                panic!("version that is newer than the latest ({version} > {latest_version})");
            }
        };
        VersionedIterator {
            nested_map: &self.nested_map,
            range,
            version,
            last_visited_key: None,
            phantom: PhantomData,
        }
    }
}

pub struct VersionedIterator<'a, K, V, R, T: ?Sized> {
    nested_map: &'a BTreeMap<K, BTreeMap<u64, Op<V>>>,
    range: R,
    version: u64,
    last_visited_key: Option<&'a K>,
    phantom: PhantomData<T>,
}

impl<'a, K, V, R, T: ?Sized> Iterator for VersionedIterator<'a, K, V, R, T>
where
    K: Borrow<T> + Ord,
    T: Ord,
    R: RangeBounds<T>,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let start = match self.last_visited_key {
            Some(key) => Bound::Excluded(key.borrow()),
            None => self.range.start_bound(),
        };

        for (key, inner_map) in self.nested_map.range((start, self.range.end_bound())) {
            if let Some((_, Op::Insert(value))) = inner_map.range(0..=self.version).last() {
                self.last_visited_key = Some(key);
                return Some((key, value));
            }
        }

        None
    }
}

impl<'a, K, V, R, T: ?Sized> DoubleEndedIterator for VersionedIterator<'a, K, V, R, T>
where
    K: Borrow<T> + Ord,
    T: Ord,
    R: RangeBounds<T>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let end = match self.last_visited_key {
            Some(key) => Bound::Excluded(key.borrow()),
            None => self.range.end_bound(),
        };

        for (key, inner_map) in self.nested_map.range((self.range.start_bound(), end)) {
            if let Some((_, Op::Insert(value))) = inner_map.range(0..=self.version).last() {
                self.last_visited_key = Some(key);
                return Some((key, value));
            }
        }

        None
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterating() {
        let mut map = VersionedMap::<&str, &str>::new();
        // apply some batches
        for batch in [
            // version: 0
            vec![
                ("larry", Op::Insert("engineer")),
                ("pumpkin", Op::Insert("cat")),
                ("donald", Op::Insert("trump")),
                ("joe", Op::Insert("biden")),
                ("satoshi", Op::Insert("nakamoto")),
            ],
            // version: 1
            vec![
                ("donald", Op::Insert("duck")),
                ("pumpkin", Op::Delete),
                ("ulfric", Op::Insert("stormcloak")),
            ],
            // version: 2
            vec![
                ("jake", Op::Insert("shepherd")),
                ("larry", Op::Insert("founder")),
                ("joe", Op::Delete),
            ],
        ] {
            map.write_batch(batch);
        }

        assert!(map.range::<_, str>(.., 0).map(|(k, v)| (*k, *v)).eq([
            ("donald", "trump"),
            ("joe", "biden"),
            ("larry", "engineer"),
            ("pumpkin", "cat"),
            ("satoshi", "nakamoto")
        ]));
        assert!(map.range::<_, str>(.., 1).map(|(k, v)| (*k, *v)).eq([
            ("donald", "duck"),
            ("joe", "biden"),
            ("larry", "engineer"),
            ("satoshi", "nakamoto"),
            ("ulfric", "stormcloak"),
        ]));
        assert!(map.range::<_, str>(.., 2).map(|(k, v)| (*k, *v)).eq([
            ("donald", "duck"),
            ("jake", "shepherd"),
            ("larry", "founder"),
            ("satoshi", "nakamoto"),
            ("ulfric", "stormcloak"),
        ]));
    }
}
