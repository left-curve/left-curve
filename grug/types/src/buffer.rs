#[cfg(feature = "metrics")]
use std::time::Instant;
use {
    crate::{Batch, MetricsIterExt, Op, Order, Record, Storage, btree_map},
    std::{
        cmp::Ordering,
        iter::{self, Peekable},
        mem,
        ops::Bound,
    },
};

pub const BUFFER_LABEL: &str = "grug.types.buffer.duration";

#[cfg(feature = "metrics")]
macro_rules! record_buffer {
    ($self:ident, $duration:ident, $operation:expr) => {
        {
            metrics::histogram!(BUFFER_LABEL, "name" => $self.name.unwrap_or("unknown"), "operation" => $operation)
                .record($duration.elapsed().as_secs_f64());
        }
    };
}

/// A key-value storage with an in-memory write buffer.
///
/// Adapted from cw-multi-test:
/// <https://github.com/CosmWasm/cw-multi-test/blob/v0.19.0/src/transactions.rs#L170-L253>
#[derive(Clone)]
pub struct Buffer<S> {
    base: S,
    pending: Batch,
    name: Option<&'static str>,
}

impl<S> Buffer<S> {
    /// Create a new buffer storage with an optional write batch.
    pub fn new(base: S, pending: Option<Batch>, name: Option<&'static str>) -> Self {
        Self {
            base,
            pending: pending.unwrap_or_default(),
            name,
        }
    }

    /// Comsume self, do not flush, just return the underlying store and the
    /// pending ops.
    pub fn disassemble(self) -> (S, Batch) {
        (self.base, self.pending)
    }
}

impl<S> Buffer<S>
where
    S: Storage,
{
    /// Flush pending ops to the underlying store.
    pub fn commit(&mut self) {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        let pending = mem::take(&mut self.pending);

        self.base.flush(pending);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "commit");
    }

    /// Consume self, flush pending ops to the underlying store, return the
    /// underlying store.
    pub fn consume(mut self) -> S {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        self.base.flush(self.pending);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "consume");

        self.base
    }
}

impl<S> Storage for Buffer<S>
where
    S: Storage + Clone,
{
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        let pending = self.pending.get(key);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "read");

        match pending {
            Some(Op::Insert(value)) => Some(value.clone()),
            Some(Op::Delete) => None,
            None => self.base.read(key),
        }
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Box::new(iter::empty());
            }
        }

        let base = self.base.scan(min, max, order);

        let min = min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));

        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        let pending_raw = self.pending.range((min, max));

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "scan");

        let pending: Box<dyn Iterator<Item = _>> = match order {
            Order::Ascending => Box::new(pending_raw),
            Order::Descending => Box::new(pending_raw.rev()),
        };

        let pending = pending.with_metrics(
            BUFFER_LABEL,
            btree_map!("operation" => "next", "name" => self.name.unwrap()),
        );

        let result = Box::new(Merged::new(base, pending, order));

        result
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        // Currently we simply iterate both keys and values, and discard the
        // values. This isn't efficient.
        // TODO: optimize this

        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        let result = Box::new(self.scan(min, max, order).map(|(k, _)| k));

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "scan_keys");

        result
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        let result = Box::new(self.scan(min, max, order).map(|(_, v)| v));

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "scan_values");

        result
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        self.pending
            .insert(key.to_vec(), Op::Insert(value.to_vec()));

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "write");
    }

    fn remove(&mut self, key: &[u8]) {
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        self.pending.insert(key.to_vec(), Op::Delete);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "remove");
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        // Find all keys within the bounds and mark them all as to be deleted.
        //
        // We use `self.scan_keys` here, which scans both the base and pending.
        //
        // We have to collect the iterator, because the iterator holds an
        // immutable reference to `self`, but `self.pending.extend` requires a
        // mutable reference, which can't coexist.
        let deletes = self
            .scan_keys(min, max, Order::Ascending)
            .map(|key| (key, Op::Delete))
            .collect::<Vec<_>>();

        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        self.pending.extend(deletes);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "remove_range");
    }

    fn flush(&mut self, batch: Batch) {
        // When we do `a.extend(b)`, while `a` and `b` have common keys, the
        // values in `b` are chosen. This is exactly what we want.
        #[cfg(feature = "metrics")]
        let duration = Instant::now();

        self.pending.extend(batch);

        #[cfg(feature = "metrics")]
        record_buffer!(self, duration, "flush");
    }
}

struct Merged<'a, B, P>
where
    B: Iterator<Item = Record>,
    P: Iterator<Item = (&'a Vec<u8>, &'a Op)>,
{
    base: Peekable<B>,
    pending: Peekable<P>,
    order: Order,
}

impl<'a, B, P> Merged<'a, B, P>
where
    B: Iterator<Item = Record>,
    P: Iterator<Item = (&'a Vec<u8>, &'a Op)>,
{
    pub fn new(base: B, pending: P, order: Order) -> Self {
        Self {
            base: base.peekable(),
            pending: pending.peekable(),
            order,
        }
    }

    fn take_pending(&mut self) -> Option<Record> {
        let (key, op) = self.pending.next()?;

        match op {
            Op::Insert(value) => Some((key.clone(), value.clone())),
            Op::Delete => self.next(),
        }
    }
}

impl<'a, B, P> Iterator for Merged<'a, B, P>
where
    B: Iterator<Item = Record>,
    P: Iterator<Item = (&'a Vec<u8>, &'a Op)>,
{
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match (self.base.peek(), self.pending.peek()) {
            (Some((base_key, _)), Some((pending_key, _))) => {
                let ordering_raw = base_key.cmp(pending_key);
                let ordering = match self.order {
                    Order::Ascending => ordering_raw,
                    Order::Descending => ordering_raw.reverse(),
                };

                match ordering {
                    Ordering::Less => self.base.next(),
                    Ordering::Equal => {
                        self.base.next();
                        self.take_pending()
                    },
                    Ordering::Greater => self.take_pending(),
                }
            },
            (None, Some(_)) => self.take_pending(),
            (Some(_), None) => self.base.next(),
            (None, None) => None,
        };

        result
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, crate::MockStorage};

    // Illustration of this test case:
    //
    // base    : 1 2 _ 4 5 6 7 _
    // pending :   D P _ _ P D 8  (P = put, D = delete)
    // merged  : 1 _ 3 4 5 6 _ 8
    fn make_test_case() -> (Buffer<MockStorage>, Vec<Record>) {
        let mut base = MockStorage::new();
        base.write(&[1], &[1]);
        base.write(&[2], &[2]);
        base.write(&[4], &[4]);
        base.write(&[5], &[5]);
        base.write(&[6], &[6]);
        base.write(&[7], &[7]);

        let mut buffer = Buffer::new(base, None, None);
        buffer.remove(&[2]);
        buffer.write(&[3], &[3]);
        buffer.write(&[6], &[255]);
        buffer.remove(&[7]);
        buffer.write(&[8], &[8]);

        let merged = vec![
            (vec![1], vec![1]),
            (vec![3], vec![3]),
            (vec![4], vec![4]),
            (vec![5], vec![5]),
            (vec![6], vec![255]),
            (vec![8], vec![8]),
        ];

        (buffer, merged)
    }

    fn collect_records(storage: &dyn Storage, order: Order) -> Vec<Record> {
        storage.scan(None, None, order).collect()
    }

    #[test]
    fn iterator_works() {
        let (buffer, mut merged) = make_test_case();
        assert_eq!(collect_records(&buffer, Order::Ascending), merged);

        merged.reverse();
        assert_eq!(collect_records(&buffer, Order::Descending), merged);
    }
}
