use {
    crate::{DefaultFamily, Family, MultiThreadedDb, Timestamped, timestamp::U64Timestamp},
    grug_types::{Defined, MaybeDefined, Undefined},
    rocksdb::{BoundColumnFamily, WriteBatch},
    std::{collections::BTreeMap, sync::Arc},
};

pub struct BatchBuilder<'a, TS: MaybeDefined<U64Timestamp>> {
    db: &'a MultiThreadedDb,
    batch: WriteBatch,
    timestamp: TS,
    cf: BTreeMap<&'static str, Arc<BoundColumnFamily<'a>>>,
}

impl<'a> BatchBuilder<'a, Undefined<U64Timestamp>> {
    pub fn new(db: &'a MultiThreadedDb) -> Self {
        Self {
            db,
            batch: WriteBatch::default(),
            timestamp: Undefined::new(),
            cf: BTreeMap::new(),
        }
    }

    pub fn with_timestamp(
        self,
        timestamp: U64Timestamp,
    ) -> BatchBuilder<'a, Defined<U64Timestamp>> {
        BatchBuilder {
            db: self.db,
            batch: WriteBatch::default(),
            timestamp: Defined::new(timestamp),
            cf: self.cf,
        }
    }
}

impl<'a> BatchBuilder<'a, Undefined<U64Timestamp>> {
    pub fn update<'b, C>(&'b mut self, family: DefaultFamily, callback: C)
    where
        C: (FnOnce(&mut BatchInner<'a, 'b>)),
    {
        let mut inner = BatchInner {
            batch: &mut self.batch,
            cf: family.cf_handle(self.db),
            timestamp: None,
        };
        callback(&mut inner);
    }
}

impl<'a> BatchBuilder<'a, Defined<U64Timestamp>> {
    pub fn update<'b, C, F>(&'b mut self, family: Family<F>, callback: C)
    where
        C: (FnOnce(&mut BatchInner<'a, 'b>)),
        F: MaybeDefined<Timestamped>,
    {
        let timestamp = if F::maybe_defined() {
            Some(self.timestamp.into_inner())
        } else {
            None
        };

        let mut inner = BatchInner {
            batch: &mut self.batch,
            cf: family.cf_handle(self.db),
            timestamp,
        };
        callback(&mut inner);
    }
}

impl<'a, T> BatchBuilder<'a, T>
where
    T: MaybeDefined<U64Timestamp>,
{
    pub fn commit(self) -> Result<(), rocksdb::Error> {
        self.db.write(self.batch)
    }
}

pub struct BatchInner<'a, 'b> {
    batch: &'b mut WriteBatch,
    cf: Arc<BoundColumnFamily<'a>>,
    timestamp: Option<U64Timestamp>,
}

impl<'a, 'b> BatchInner<'a, 'b> {
    pub fn put<K, V>(&mut self, key: K, value: V)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        if let Some(timestamp) = self.timestamp {
            self.batch.put_cf_with_ts(&self.cf, key, timestamp, value);
        } else {
            self.batch.put_cf(&self.cf, key, value);
        }
    }

    pub fn delete<K>(&mut self, key: K)
    where
        K: AsRef<[u8]>,
    {
        if let Some(timestamp) = self.timestamp {
            self.batch.delete_cf_with_ts(&self.cf, key, timestamp);
        } else {
            self.batch.delete_cf(&self.cf, key);
        }
    }
}
