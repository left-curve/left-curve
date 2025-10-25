use {
    crate::{
        MultiThreadedDb,
        timestamp::{U64Comparator, U64Timestamp},
    },
    grug_types::{Defined, MaybeDefined, Order, Undefined},
    rocksdb::{BoundColumnFamily, IteratorMode, Options, ReadOptions},
    std::{marker::PhantomData, sync::Arc},
};

/// Alias for a **plain** (non-timestamped) column family.
pub type PlainCf = ColumnFamily<Undefined<Versioned>>;

/// Alias for a **timestamp-aware** (versioned) column family.
pub type VersionedCf = ColumnFamily<Defined<Versioned>>;

/// Column-family descriptor parameterized by whether the CF is timestamp-aware
/// (`Family<Defined<Timestamped>>`) or plain (`Family<Undefined<Timestamped>>`).
///
/// This type does **not** own a DB handle; it only carries the CF name and
/// determines options (e.g. enabling the timestamp comparator) at open time.
#[derive(Clone, Debug)]
pub struct ColumnFamily<T: MaybeDefined<Versioned>> {
    pub(crate) name: &'static str,
    _options: PhantomData<T>,
}

impl<T> ColumnFamily<T>
where
    T: MaybeDefined<Versioned>,
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
            panic!("failed to find column family {}", self.name);
        })
    }

    fn iterator_bounds(
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

impl VersionedCf {
    pub fn read(&self, db: &MultiThreadedDb, version: u64, key: &[u8]) -> Option<Vec<u8>> {
        db.get_cf_opt(&self.cf_handle(db), key, &self.read_options(version))
            .unwrap_or_else(|err| {
                panic!("failed to read from column family {}: {err}", self.name);
            })
    }

    pub fn iter<'a>(
        &'a self,
        db: &'a MultiThreadedDb,
        version: u64,
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
                    panic!("failed to iterate in column family {}: {err}", self.name);
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }

    pub fn read_options(&self, version: u64) -> ReadOptions {
        let mut opts = ReadOptions::default();

        opts.set_timestamp(U64Timestamp::from(version));

        opts
    }

    fn iteration_options(
        &self,
        version: u64,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
    ) -> ReadOptions {
        self.iterator_bounds(self.read_options(version), min, max)
    }
}

impl PlainCf {
    pub fn read(&self, db: &MultiThreadedDb, key: &[u8]) -> Option<Vec<u8>> {
        db.get_cf(&self.cf_handle(db), key).unwrap_or_else(|err| {
            panic!("failed to read from column family {}: {err}", self.name);
        })
    }

    pub fn iter<'a>(
        &'a self,
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
                    panic!("failed to iterate in column family {}: {err}", self.name);
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }
}

/// Zero-sized marker used in the type-state to indicate a timestamp-aware CF.
#[derive(Clone, Debug)]
pub struct Versioned;

#[inline]
fn into_iterator_mode(order: Order) -> IteratorMode<'static> {
    match order {
        Order::Ascending => IteratorMode::Start,
        Order::Descending => IteratorMode::End,
    }
}
