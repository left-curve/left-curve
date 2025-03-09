use {
    crate::{GAS_COSTS, GasTracker},
    grug_storage::{Codec, IndexedMap, Item, Map, PrimaryKey},
    grug_types::{Bound, Order, Record, StdResult, Storage},
};

// ---------------------------------- storage ----------------------------------

pub trait MeteredStorage {
    fn read_with_gas(&self, gas_tracker: GasTracker, key: &[u8]) -> StdResult<Option<Vec<u8>>>;

    fn scan_with_gas<'a>(
        &'a self,
        gas_tracker: GasTracker,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<Record>> + 'a>>;
}

impl<S> MeteredStorage for S
where
    S: Storage,
{
    fn read_with_gas(&self, gas_tracker: GasTracker, key: &[u8]) -> StdResult<Option<Vec<u8>>> {
        let maybe_data = self.read(key);

        match &maybe_data {
            Some(data) => {
                gas_tracker.consume(GAS_COSTS.db_read.cost(data.len()), "db_read/found")?;
            },
            None => {
                gas_tracker.consume(GAS_COSTS.db_read.cost(0), "db_read/not_found")?;
            },
        }

        Ok(maybe_data)
    }

    fn scan_with_gas<'a>(
        &'a self,
        gas_tracker: GasTracker,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<Record>> + 'a>> {
        // Gas cost for creating an iterator.
        gas_tracker.consume(GAS_COSTS.db_scan, "db_scan")?;

        Ok(Box::new(self.scan(min, max, order).metered(gas_tracker)))
    }
}

// ----------------------------------- item ------------------------------------

pub trait MeteredItem<T> {
    fn load_with_gas(&self, storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<T>;
}

impl<T, C> MeteredItem<T> for Item<'_, T, C>
where
    C: Codec<T>,
{
    fn load_with_gas(&self, storage: &dyn Storage, gas_tracker: GasTracker) -> StdResult<T> {
        let data_raw = self.load_raw(storage)?;

        gas_tracker.consume(GAS_COSTS.db_read.cost(data_raw.len()), "db_read/found")?;

        C::decode(&data_raw)
    }
}

// ------------------------------------ map ------------------------------------

pub trait MeteredMap<K, T>
where
    K: PrimaryKey,
{
    fn load_with_gas(&self, storage: &dyn Storage, gas_tracker: GasTracker, key: K)
    -> StdResult<T>;

    fn has_with_gas(
        &self,
        storage: &dyn Storage,
        gas_tracker: GasTracker,
        key: K,
    ) -> StdResult<bool>;

    fn range_with_gas<'b>(
        &self,
        storage: &'b dyn Storage,
        gas_tracker: GasTracker,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b>>
    where
        T: 'b;

    fn save_with_gas(
        &self,
        storage: &mut dyn Storage,
        gas_tracker: GasTracker,
        key: K,
        value: &T,
    ) -> StdResult<()>;
}

impl<K, T, C> MeteredMap<K, T> for Map<'_, K, T, C>
where
    K: PrimaryKey,
    C: Codec<T>,
{
    fn load_with_gas(
        &self,
        storage: &dyn Storage,
        gas_tracker: GasTracker,
        key: K,
    ) -> StdResult<T> {
        let data_raw = self.path(key).load_raw(storage)?;

        gas_tracker.consume(GAS_COSTS.db_read.cost(data_raw.len()), "db_read/found")?;

        C::decode(&data_raw)
    }

    fn has_with_gas(
        &self,
        storage: &dyn Storage,
        gas_tracker: GasTracker,
        key: K,
    ) -> StdResult<bool> {
        match self.path(key).may_load_raw(storage) {
            Some(data) => {
                gas_tracker.consume(GAS_COSTS.db_read.cost(data.len()), "db_read/found")?;
                Ok(true)
            },
            None => {
                gas_tracker.consume(GAS_COSTS.db_read.cost(0), "db_read/not_found")?;
                Ok(false)
            },
        }
    }

    fn range_with_gas<'b>(
        &self,
        storage: &'b dyn Storage,
        gas_tracker: GasTracker,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b>>
    where
        T: 'b,
    {
        // Gas cost for creating an iterator.
        gas_tracker.consume(GAS_COSTS.db_scan, "db_scan")?;

        let iter = self
            .range_raw(storage, min, max, order)
            .metered(gas_tracker)
            .map(|record| {
                let (k_raw, v_raw) = record?;
                let k = K::from_slice(&k_raw)?;
                let v = C::decode(&v_raw)?;
                Ok((k, v))
            });

        Ok(Box::new(iter))
    }

    fn save_with_gas(
        &self,
        storage: &mut dyn Storage,
        gas_tracker: GasTracker,
        key: K,
        value: &T,
    ) -> StdResult<()> {
        let data_raw = C::encode(value)?;
        let path = self.path(key);

        let gas_cost = GAS_COSTS
            .db_write
            .cost(data_raw.len() + path.storage_key().len());

        // Charge gas before writing the data, such that if run out of gas,
        // the data isn't written.
        gas_tracker.consume(gas_cost, "db_write")?;

        path.save_raw(storage, &data_raw);

        Ok(())
    }
}

// ------------------------------------ index map ------------------------------------

impl<K, T, I, C> MeteredMap<K, T> for IndexedMap<'_, K, T, I, C>
where
    K: PrimaryKey,
    C: Codec<T>,
{
    fn load_with_gas(
        &self,
        storage: &dyn Storage,
        gas_tracker: GasTracker,
        key: K,
    ) -> StdResult<T> {
        self.primary.load_with_gas(storage, gas_tracker, key)
    }

    fn has_with_gas(
        &self,
        storage: &dyn Storage,
        gas_tracker: GasTracker,
        key: K,
    ) -> StdResult<bool> {
        self.primary.has_with_gas(storage, gas_tracker, key)
    }

    fn range_with_gas<'b>(
        &self,
        storage: &'b dyn Storage,
        gas_tracker: GasTracker,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b>>
    where
        T: 'b,
    {
        self.primary
            .range_with_gas(storage, gas_tracker, min, max, order)
    }

    fn save_with_gas(
        &self,
        storage: &mut dyn Storage,
        gas_tracker: GasTracker,
        key: K,
        value: &T,
    ) -> StdResult<()> {
        // TODO: this implementation doesn't account for gas cost of writing to
        // the index set.
        self.primary.save_with_gas(storage, gas_tracker, key, value)
    }
}

// --------------------------------- iterator ----------------------------------

pub trait MeteredIterator: Sized {
    fn metered(self, gas_tracker: GasTracker) -> MeteredIter<Self> {
        MeteredIter {
            iter: self,
            gas_tracker,
        }
    }
}

impl<I> MeteredIterator for I where I: Iterator<Item = Record> {}

pub struct MeteredIter<I> {
    iter: I,
    gas_tracker: GasTracker,
}

impl<I> Iterator for MeteredIter<I>
where
    I: Iterator<Item = Record>,
{
    type Item = StdResult<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((k_raw, v_raw)) = self.iter.next() {
            // A record is found. We charge both the cost for advancing the
            // iterator (`db_next`) and for reading the record (`db_read`).
            let cost = GAS_COSTS.db_next + GAS_COSTS.db_read.cost(k_raw.len() + v_raw.len());

            match self.gas_tracker.consume(cost, "db_next/found") {
                Ok(()) => Some(Ok((k_raw, v_raw))),
                Err(err) => Some(Err(err)),
            }
        } else {
            // No record is found; iterator has reached its end.
            // Charge only the cost for advanding iterator.
            let cost = GAS_COSTS.db_next;

            match self.gas_tracker.consume(cost, "db_next/not_found") {
                Ok(()) => None,
                Err(err) => Some(Err(err)),
            }
        }
    }
}
