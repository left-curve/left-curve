use {
    crate::{Borsh, Codec, Item, Key, Map},
    grug_types::{Number, StdResult, Storage},
};

/// A single number that is monotonically incremented by the given step size.
///
/// In internally, this is an abstraction over an [`Item`](crate::Item).
pub struct Counter<'a, T, C = Borsh>
where
    C: Codec<T>,
{
    item: Item<'a, T, C>,
    base: T,
    step: T,
}

impl<'a, T, C> Counter<'a, T, C>
where
    T: Number + Copy,
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str, base: T, step: T) -> Self {
        Self {
            item: Item::new(storage_key),
            base,
            step,
        }
    }

    /// Load the current counter value.
    pub fn current(&self, storage: &dyn Storage) -> StdResult<T> {
        self.item
            .may_load(storage)
            .map(|maybe_value| maybe_value.unwrap_or(self.base))
    }

    /// Increment the value by the step size; return the values before and after
    /// incrementing.
    pub fn increment(&self, storage: &mut dyn Storage) -> StdResult<(T, T)> {
        let old_value = self.current(storage)?;
        let new_value = old_value.checked_add(self.step)?;

        self.item.save(storage, &new_value)?;

        Ok((old_value, new_value))
    }
}

/// A single number under each key, that is monotonically incremented by the
/// given step size.
///
/// Internally, this is an abstraction over a [`Map`](crate::Map).
pub struct Counters<'a, K, T, C = Borsh>
where
    C: Codec<T>,
{
    map: Map<'a, K, T, C>,
    base: T,
    step: T,
}

impl<'a, K, T, C> Counters<'a, K, T, C>
where
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str, base: T, step: T) -> Self {
        Self {
            map: Map::new(storage_key),
            base,
            step,
        }
    }
}
impl<'a, K, T, C> Counters<'a, K, T, C>
where
    K: Key + Copy,
    T: Number + Copy,
    C: Codec<T>,
{
    /// Load the current counter value under the given key.
    pub fn current(&self, storage: &dyn Storage, key: K) -> StdResult<T> {
        self.map
            .may_load(storage, key)
            .map(|maybe_value| maybe_value.unwrap_or(self.base))
    }

    /// Increment the value under the given key by the step size; return the
    /// values before and after incrementing.
    pub fn increment(&self, storage: &mut dyn Storage, key: K) -> StdResult<(T, T)> {
        let old_value = self.current(storage, key)?;
        let new_value = old_value.checked_add(self.step)?;

        self.map.save(storage, key, &new_value)?;

        Ok((old_value, new_value))
    }
}

#[cfg(test)]
mod tests {

    use {
        super::Counter,
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::{Dec128, Int128, MockStorage, Number, NumberConst, Uint128, Uint512},
        std::{fmt::Debug, marker::PhantomData},
        test_case::test_case,
    };

    #[test_case(PhantomData::<u8>; "u8")]
    #[test_case(PhantomData::<Uint128>; "uint128")]
    #[test_case(PhantomData::<Uint512>; "uint512")]
    #[test_case(PhantomData::<Int128>; "int128")]
    #[test_case(PhantomData::<Dec128>; "dec128")]
    fn counter<T>(_p: PhantomData<T>)
    where
        T: BorshSerialize + BorshDeserialize + NumberConst + Number + PartialEq + Debug,
    {
        let mut storage = MockStorage::new();
        let counter = Counter::<T>::new("counter");

        counter.load(&storage).unwrap_err();

        let mut asserter = T::ZERO;

        counter.initialize(&mut storage).unwrap();
        assert_eq!(counter.load(&storage).unwrap(), asserter);

        asserter = asserter.checked_add(T::ONE).unwrap();
        assert_eq!(counter.increment(&mut storage).unwrap(), asserter);
        assert_eq!(counter.load(&storage).unwrap(), asserter);

        asserter = asserter.checked_add(T::ONE).unwrap();
        assert_eq!(counter.increment(&mut storage).unwrap(), asserter);
        assert_eq!(counter.load(&storage).unwrap(), asserter);
    }
}
