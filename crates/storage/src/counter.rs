use {
    crate::{Borsh, Codec, Item, Map, PrimaryKey},
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
    K: PrimaryKey + Copy,
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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::Counter,
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::{Dec128, Int128, MockStorage, Number, NumberConst, Uint128, Uint512},
        std::{fmt::Debug, str::FromStr},
        test_case::test_case,
    };

    #[test_case(
        0_u8,
        1_u8;
        "u8"
    )]
    #[test_case(
        Uint128::ZERO,
        Uint128::TEN;
        "uint128"
    )]
    #[test_case(
        Uint512::ONE,
        Uint512::ONE;
        "uint512"
    )]
    #[test_case(
        Int128::new_negative(Uint128::ONE),
        Int128::new_negative(Uint128::ONE);
        "int128"
    )]
    #[test_case(
        Dec128::from_str("0.5").unwrap(),
        Dec128::from_str("1.5").unwrap();
        "dec128"
    )]
    fn counter_works<T>(base: T, increment: T)
    where
        T: BorshSerialize + BorshDeserialize + NumberConst + Number + PartialEq + Debug + Copy,
    {
        let counter = Counter::<T>::new("counter", base, increment);

        let mut storage = MockStorage::new();
        let mut current = base;
        let mut next = current.checked_add(increment).unwrap();

        for _ in 0..10 {
            assert_eq!(counter.current(&storage).unwrap(), current);
            assert_eq!(counter.increment(&mut storage).unwrap(), (current, next));

            current = next;
            next = next.checked_add(increment).unwrap();
        }
    }
}
