use {
    crate::{
        MultiThreadedDb,
        timestamp::{U64Comparator, U64Timestamp},
    },
    grug_types::{Defined, MaybeDefined, Order, Undefined},
    rocksdb::{BoundColumnFamily, IteratorMode, Options, ReadOptions},
    std::{marker::PhantomData, sync::Arc},
};

pub type DefaultFamily = Family<Undefined<Timestamped>>;

pub type TimestampedFamily = Family<Defined<Timestamped>>;

#[derive(Clone, Debug)]
pub struct Family<T: MaybeDefined<Timestamped>> {
    pub(crate) name: &'static str,
    _options: PhantomData<T>,
}

impl<T> Family<T>
where
    T: MaybeDefined<Timestamped>,
{
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _options: PhantomData,
        }
    }

    pub fn open_opt(&self) -> (&'static str, Options) {
        let cf = if T::maybe_defined() {
            let mut opts = Options::default();
            // Must use a timestamp-enabled comparator
            opts.set_comparator_with_ts(
                U64Comparator::NAME,
                U64Timestamp::SIZE,
                Box::new(U64Comparator::compare),
                Box::new(U64Comparator::compare_ts),
                Box::new(U64Comparator::compare_without_ts),
            );
            opts
        } else {
            Options::default()
        };

        (self.name, cf)
    }

    pub fn cf_handle<'a>(&self, db: &'a MultiThreadedDb) -> Arc<BoundColumnFamily<'a>> {
        db.cf_handle(self.name).unwrap_or_else(|| {
            panic!("failed to find column family");
        })
    }

    fn iterator_bounds<'a>(
        &self,
        mut opt: ReadOptions,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
    ) -> ReadOptions {
        if let Some(min) = min {
            opt.set_iterate_lower_bound(min);
        }
        if let Some(max) = max {
            opt.set_iterate_upper_bound(max);
        }
        opt
    }
}

impl TimestampedFamily {
    pub fn read(&self, db: &MultiThreadedDb, version: Option<u64>, key: &[u8]) -> Option<Vec<u8>> {
        db.get_cf_opt(&self.cf_handle(db), key, &self.read_options(version))
            .unwrap_or_else(|err| {
                panic!("failed to read from column family: {err}");
            })
    }

    pub fn iter<'a>(
        &self,
        db: &'a MultiThreadedDb,
        version: Option<u64>,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let iter = db
            .iterator_cf_opt(
                &self.cf_handle(db),
                self.iteration_options(version, min, max),
                into_iterator_mode(order),
            )
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in column family: {err}");
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }

    pub fn read_options(&self, version: Option<u64>) -> ReadOptions {
        let mut opts = ReadOptions::default();
        if let Some(version) = version {
            opts.set_timestamp(U64Timestamp::from(version));
        }
        opts
    }

    fn iteration_options(
        &self,
        version: Option<u64>,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
    ) -> ReadOptions {
        self.iterator_bounds(self.read_options(version), min, max)
    }
}

impl DefaultFamily {
    pub fn read(&self, db: &MultiThreadedDb, key: &[u8]) -> Option<Vec<u8>> {
        db.get_cf(&self.cf_handle(db), key).unwrap_or_else(|err| {
            panic!("failed to read from column family: {err}");
        })
    }

    pub fn iter<'a>(
        &self,
        db: &'a MultiThreadedDb,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let iter = db
            .iterator_cf_opt(
                &self.cf_handle(db),
                self.iterator_bounds(Default::default(), min, max),
                into_iterator_mode(order),
            )
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in column family: {err}");
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }
}

#[derive(Clone, Debug)]
pub struct Timestamped;

#[inline]
fn into_iterator_mode(order: Order) -> IteratorMode<'static> {
    match order {
        Order::Ascending => IteratorMode::Start,
        Order::Descending => IteratorMode::End,
    }
}
